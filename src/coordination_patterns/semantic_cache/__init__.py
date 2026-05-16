"""Semantic Intent Cache.

Before extracting intent via LLM, check a local cache of previously seen
requests. If a sufficiently similar query exists, return the cached intent
immediately — bypassing the LLM entirely. Every successful extraction self-populates
the cache for future hits.

Supports two persistence backends:
- memory (default): in-memory, lost on process exit
- sqlite: persists to disk via SQLite WAL mode
"""

from __future__ import annotations

import time
from typing import Any

from pydantic import BaseModel

from coordination_patterns.capability_router.pattern import RoutingIntent
from coordination_patterns.semantic_cache.store import (
    CacheStoreProtocol,
    MemoryCacheStore,
    SqliteCacheStore,
    _Row,
)
from coordination_patterns.semantic_cache.utils import cosine_similarity


class CachedEntry(BaseModel):
    """A single entry in the semantic cache."""

    query: str
    embedding: list[float]
    intent: RoutingIntent
    created_at: float = 0.0
    hit_count: int = 0

    def __post_init__(self):
        if self.created_at == 0.0:
            self.created_at = time.time()


class SemanticCache:
    """Semantic cache for routing intents with pluggable persistence.

    Stores (query, embedding, intent) tuples. On lookup, computes cosine
    similarity between the query embedding and all stored embeddings. Returns
    the cached intent if the best match exceeds the threshold.
    """

    def __init__(
        self,
        threshold: float = 0.92,
        max_size: int = 1000,
        store: str = "memory",
        store_path: str | None = None,
    ) -> None:
        """
        Args:
            threshold: Minimum cosine similarity to count as a cache hit.
            max_size: Maximum number of entries before LRU eviction.
            store: Persistence backend — "memory" or "sqlite".
            store_path: Path to SQLite database (used when store="sqlite").
        """
        self.threshold = threshold
        self.max_size = max_size

        if store == "memory":
            self._backend: CacheStoreProtocol = MemoryCacheStore()
        elif store == "sqlite":
            self._backend = SqliteCacheStore(store_path)
        else:
            raise ValueError(
                f"Unknown store type '{store}'. Expected 'memory' or 'sqlite'."
            )

    # -- public API --------------------------------------------------------

    def lookup(self, embedding: list[float]) -> RoutingIntent | None:
        """Find the most similar cached entry.

        Returns the cached intent if similarity >= threshold, else None.
        """
        rows = self._backend.get_all()
        if not rows:
            return None

        best_score = -1.0
        best_row: _Row | None = None

        for row in rows:
            score = cosine_similarity(embedding, row.embedding)
            if score > best_score:
                best_score = score
                best_row = row

        if best_row is not None and best_score >= self.threshold:
            self._backend.increment_hit(best_row.embedding)
            return best_row.to_intent()

        return None

    def store(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        """Add an entry. Evict least-recently-hit if at max_size."""
        row = _Row(
            query=query,
            embedding=embedding,
            intent_dict={
                "action": intent.action,
                "resource": intent.resource,
                "parameters": intent.parameters,
            },
        )
        self._backend.put(row)
        self._evict_if_needed()

    @property
    def size(self) -> int:
        """Number of entries in the cache."""
        return len(self._backend.get_all())

    def clear(self) -> None:
        """Remove all entries."""
        self._backend.clear()

    def close(self) -> None:
        """Release any underlying resources (e.g. DB connections)."""
        self._backend.close()

    # -- internal helpers --------------------------------------------------

    def _evict_if_needed(self) -> None:
        """Evict the lowest-hit entry while size exceeds max_size."""
        while len(self._backend.get_all()) > self.max_size:
            self._backend.evict_lowest()

    # -- backward-compat shim for unit tests that inspect _entries ----------

    @property
    def _entries(self) -> list[CachedEntry]:
        """Expose rows as CachedEntry models (kept for test compat)."""
        result: list[CachedEntry] = []
        for row in self._backend.get_all():
            result.append(
                CachedEntry(
                    query=row.query,
                    embedding=row.embedding,
                    intent=row.to_intent(),
                    created_at=row.created_at,
                    hit_count=row.hit_count,
                )
            )
        return result
