"""Persistent cache stores for the semantic cache.

Two backends:
- MemoryCacheStore — in-memory (current behaviour, no persistence)
- SqliteCacheStore — SQLite-backed file cache (survives process restart)
"""

from __future__ import annotations

import json
import os
import sqlite3
from abc import ABC, abstractmethod
from typing import Any

from coordination_patterns.capability_router.pattern import RoutingIntent


class CacheStore(ABC):
    """Abstract persistence layer for semantic cache entries."""

    @abstractmethod
    def get_all(self) -> list[tuple[str, list[float], RoutingIntent, float, int]]:
        """Return all entries as (query, embedding, intent, created_at, hit_count)."""

    @abstractmethod
    def add(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        """Insert a new entry."""

    @abstractmethod
    def clear(self) -> None:
        """Remove all entries."""

    @abstractmethod
    def update_hit(self, query: str, new_hit_count: int) -> None:
        """Update the hit_count for a given query."""

    @abstractmethod
    def evict(self, keep_n: int) -> None:
        """Keep only the keep_n entries with the highest hit_count."""

    def close(self) -> None:
        """Release any underlying resources. Default no-op."""


class MemoryCacheStore(CacheStore):
    """In-memory list-backed store."""

    def __init__(self) -> None:
        self._entries: list[tuple[str, list[float], RoutingIntent, float, int]] = []

    def get_all(self) -> list[tuple[str, list[float], RoutingIntent, float, int]]:
        return list(self._entries)

    def add(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        import time

        self._entries.append((query, embedding, intent, time.time(), 0))

    def clear(self) -> None:
        self._entries.clear()

    def update_hit(self, query: str, new_hit_count: int) -> None:
        for i, (q, emb, intent, created, hits) in enumerate(self._entries):
            if q == query:
                self._entries[i] = (q, emb, intent, created, new_hit_count)
                break

    def evict(self, keep_n: int) -> None:
        self._entries.sort(key=lambda e: e[4])
        if len(self._entries) > keep_n:
            self._entries = self._entries[-keep_n:]


class SqliteCacheStore(CacheStore):
    """SQLite-backed store with WAL mode for persistence."""

    def __init__(self, db_path: str) -> None:
        self._db_path = db_path
        os.makedirs(os.path.dirname(db_path), exist_ok=True)
        self._conn = sqlite3.connect(db_path)
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._init_db()

    def _init_db(self) -> None:
        self._conn.execute(
            """
            CREATE TABLE IF NOT EXISTS cache_entries (
                query       TEXT PRIMARY KEY,
                embedding   TEXT NOT NULL,
                intent      TEXT NOT NULL,
                created_at  REAL NOT NULL,
                hit_count   INTEGER NOT NULL DEFAULT 0
            )
            """
        )
        self._conn.commit()

    def _row_to_tuple(
        self, row: tuple[str, str, str, float, int]
    ) -> tuple[str, list[float], RoutingIntent, float, int]:
        query, emb_json, intent_json, created_at, hit_count = row
        embedding = json.loads(emb_json)
        intent_data = json.loads(intent_json)
        intent = RoutingIntent(**intent_data)
        return query, embedding, intent, created_at, hit_count

    def get_all(self) -> list[tuple[str, list[float], RoutingIntent, float, int]]:
        cursor = self._conn.execute("SELECT query, embedding, intent, created_at, hit_count FROM cache_entries")
        return [self._row_to_tuple(r) for r in cursor.fetchall()]

    def add(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        import time

        emb_json = json.dumps(embedding)
        intent_json = json.dumps(intent.model_dump())
        self._conn.execute(
            """
            INSERT OR REPLACE INTO cache_entries (query, embedding, intent, created_at, hit_count)
            VALUES (?, ?, ?, ?, 0)
            """,
            (query, emb_json, intent_json, time.time()),
        )
        self._conn.commit()

    def clear(self) -> None:
        self._conn.execute("DELETE FROM cache_entries")
        self._conn.commit()

    def update_hit(self, query: str, new_hit_count: int) -> None:
        self._conn.execute(
            "UPDATE cache_entries SET hit_count = ? WHERE query = ?",
            (new_hit_count, query),
        )
        self._conn.commit()

    def evict(self, keep_n: int) -> None:
        self._conn.execute(
            """
            DELETE FROM cache_entries
            WHERE query NOT IN (
                SELECT query FROM cache_entries ORDER BY hit_count DESC LIMIT ?
            )
            """,
            (keep_n,),
        )
        self._conn.commit()

    def close(self) -> None:
        if self._conn:
            self._conn.close()
