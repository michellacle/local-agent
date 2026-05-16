"""Persistence backends for the semantic cache.

Two store implementations are provided:
- MemoryCacheStore: in-memory (default, matches prior behavior)
- SqliteCacheStore: SQLite-backed with WAL mode for durability
"""

from __future__ import annotations

import json
import os
import sqlite3
import time
from abc import ABC, abstractmethod
from typing import Any

from coordination_patterns.capability_router.pattern import RoutingIntent


# ---------------------------------------------------------------------------
# Internal data carrier (kept out of the public API to avoid coupling to
# Pydantic in the persistence layer).
# ---------------------------------------------------------------------------

class _Row:
    """Internal representation of a cache row used by stores."""

    __slots__ = ("query", "embedding", "intent_dict", "created_at", "hit_count")

    def __init__(
        self,
        query: str,
        embedding: list[float],
        intent_dict: dict[str, Any],
        created_at: float | None = None,
        hit_count: int = 0,
    ) -> None:
        self.query = query
        self.embedding = embedding
        self.intent_dict = intent_dict
        self.created_at = created_at if created_at is not None else time.time()
        self.hit_count = hit_count

    def to_intent(self) -> RoutingIntent:
        return RoutingIntent(**self.intent_dict)


# ---------------------------------------------------------------------------
# Protocol
# ---------------------------------------------------------------------------

class CacheStoreProtocol(ABC):
    """Abstract interface for semantic-cache persistence back-ends."""

    @abstractmethod
    def get_all(self) -> list[_Row]:
        """Return every stored row."""

    @abstractmethod
    def put(self, row: _Row) -> None:
        """Append a new row."""

    @abstractmethod
    def increment_hit(self, embedding: list[float]) -> None:
        """Increment hit_count for the row whose embedding matches."""

    @abstractmethod
    def evict_lowest(self) -> None:
        """Remove the row with the lowest hit_count."""

    @abstractmethod
    def clear(self) -> None:
        """Remove all rows."""

    @abstractmethod
    def close(self) -> None:
        """Release resources (connections, files, …)."""


# ---------------------------------------------------------------------------
# Memory back-end
# ---------------------------------------------------------------------------

class MemoryCacheStore(CacheStoreProtocol):
    """In-memory cache store (original behavior)."""

    def __init__(self) -> None:
        self._rows: list[_Row] = []

    def get_all(self) -> list[_Row]:
        return list(self._rows)

    def put(self, row: _Row) -> None:
        self._rows.append(row)

    def increment_hit(self, embedding: list[float]) -> None:
        for row in self._rows:
            if row.embedding == embedding:
                row.hit_count += 1
                return

    def evict_lowest(self) -> None:
        if self._rows:
            min_idx = min(range(len(self._rows)), key=lambda i: self._rows[i].hit_count)
            self._rows.pop(min_idx)

    def clear(self) -> None:
        self._rows.clear()

    def close(self) -> None:
        pass


# ---------------------------------------------------------------------------
# SQLite back-end
# ---------------------------------------------------------------------------

class SqliteCacheStore(CacheStoreProtocol):
    """SQLite-backed cache store using WAL mode for durability.

    Schema
    ------
    cache_entries (
        rowid       INTEGER PRIMARY KEY AUTOINCREMENT,
        query       TEXT NOT NULL,
        embedding   TEXT NOT NULL,   -- JSON array of floats
        intent      TEXT NOT NULL,   -- JSON object (action, resource, parameters)
        created_at  REAL NOT NULL,
        hit_count   INTEGER NOT NULL DEFAULT 0
    )
    """

    _DEFAULT_DIR = os.path.expanduser("~/.local/share/coordination-patterns")
    _DEFAULT_DB = os.path.join(_DEFAULT_DIR, "cache.db")

    def __init__(self, db_path: str | None = None) -> None:
        self._db_path: str = db_path or SqliteCacheStore._DEFAULT_DB
        self._conn: sqlite3.Connection | None = None
        self._open()

    # -- connection helpers ------------------------------------------------

    def _open(self) -> None:
        os.makedirs(os.path.dirname(self._db_path), exist_ok=True)
        self._conn = sqlite3.connect(self._db_path)
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._conn.execute("PRAGMA synchronous=NORMAL")
        self._conn.execute(
            """
            CREATE TABLE IF NOT EXISTS cache_entries (
                rowid       INTEGER PRIMARY KEY AUTOINCREMENT,
                query       TEXT    NOT NULL,
                embedding   TEXT    NOT NULL,
                intent      TEXT    NOT NULL,
                created_at  REAL    NOT NULL,
                hit_count   INTEGER NOT NULL DEFAULT 0
            )
            """
        )
        self._conn.commit()

    def _ensure(self) -> sqlite3.Connection:
        if self._conn is None:
            self._open()
        assert self._conn is not None
        return self._conn

    # -- protocol ---------------------------------------------------------

    def get_all(self) -> list[_Row]:
        cur = self._ensure().cursor()
        cur.execute(
            "SELECT query, embedding, intent, created_at, hit_count "
            "FROM cache_entries ORDER BY hit_count DESC"
        )
        rows: list[_Row] = []
        for q, emb, intent, ts, hc in cur.fetchall():
            rows.append(
                _Row(
                    query=q,
                    embedding=json.loads(emb),
                    intent_dict=json.loads(intent),
                    created_at=ts,
                    hit_count=hc,
                )
            )
        return rows

    def put(self, row: _Row) -> None:
        self._ensure().execute(
            "INSERT INTO cache_entries (query, embedding, intent, created_at, hit_count) "
            "VALUES (?, ?, ?, ?, ?)",
            (
                row.query,
                json.dumps(row.embedding),
                json.dumps(row.intent_dict),
                row.created_at,
                row.hit_count,
            ),
        )
        self._ensure().commit()

    def increment_hit(self, embedding: list[float]) -> None:
        self._ensure().execute(
            "UPDATE cache_entries SET hit_count = hit_count + 1 "
            "WHERE embedding = ?",
            (json.dumps(embedding),),
        )
        self._ensure().commit()

    def evict_lowest(self) -> None:
        self._ensure().execute(
            "DELETE FROM cache_entries WHERE rowid = ("
            "  SELECT rowid FROM cache_entries ORDER BY hit_count ASC LIMIT 1"
            ")"
        )
        self._ensure().commit()

    def clear(self) -> None:
        self._ensure().execute("DELETE FROM cache_entries")
        self._ensure().commit()

    def close(self) -> None:
        if self._conn is not None:
            self._conn.close()
            self._conn = None
