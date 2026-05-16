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
from pydantic import BaseModel

from coordination_patterns.capability_router.pattern import RoutingIntent
from coordination_patterns.semantic_cache.store import (
    CacheStore,
    MemoryCacheStore,
    SqliteCacheStore,
)
from coordination_patterns.semantic_cache.utils import cosine_similarity


class CachedEntry(BaseModel):
    """A single entry in the semantic cache."""

    query: str
    embedding: list[float]
    intent: RoutingIntent
    created_at: float = 0.0
    hit_count: int = 0

    def __post_init__(self) -> None:
        if self.created_at == 0.0:
            self.created_at = time.time()


class SemanticCache:
    """In-memory semantic cache for routing intents with pluggable persistence.

    Stores (query, embedding, intent) tuples. On lookup, computes cosine
    similarity between the query embedding and all stored embeddings. Returns
    the cached intent if the best match exceeds the threshold.

    Args:
        threshold: Minimum cosine similarity to count as a cache hit.
        max_size: Maximum number of entries before LRU eviction.
        store: Persistence backend — "memory" or "sqlite".
        store_path: Path to SQLite database (used when store="sqlite").
    """

    def __init__(
        self,
        threshold: float = 0.92,
        max_size: int = 1000,
        store: str = "memory",
        store_path: str | None = None,
    ) -> None:
        self.threshold = threshold
        self.max_size = max_size

        if store == "memory":
            self._backend: CacheStore = MemoryCacheStore()
        elif store == "sqlite":
            if store_path is None:
                import os

                store_path = os.path.expanduser("~/.local/share/coordination-patterns/cache.db")
            self._backend = SqliteCacheStore(store_path)
        else:
            raise ValueError(
                f"Unknown store type '{store}'. Expected 'memory' or 'sqlite'."
            )

        # In-memory index: list of CachedEntry objects that mirrors the backend.
        # This keeps the cosine similarity lookup fast and simple.
        self._entries: list[CachedEntry] = []
        self._refresh_index()

    def _refresh_index(self) -> None:
        """Reload in-memory index from the backend."""
        rows = self._backend.get_all()
        self._entries = []
        for query, embedding, intent, created_at, hit_count in rows:
            entry = CachedEntry(
                query=query,
                embedding=embedding,
                intent=intent,
                created_at=created_at,
                hit_count=hit_count,
            )
            self._entries.append(entry)

    def lookup(self, embedding: list[float]) -> RoutingIntent | None:
        """Find the most similar cached entry.

        Returns the cached intent if similarity >= threshold, else None.
        """
        best_score = -1.0
        best_entry: CachedEntry | None = None

        for entry in self._entries:
            score = cosine_similarity(embedding, entry.embedding)
            if score > best_score:
                best_score = score
                best_entry = entry

        if best_entry and best_score >= self.threshold:
            best_entry.hit_count += 1
            # Persist the hit count update
            self._backend.update_hit(best_entry.query, best_entry.hit_count)
            return best_entry.intent

        return None

    def store(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        """Add an entry. Evict least-recently-hit if at max_size."""
        entry = CachedEntry(query=query, embedding=embedding, intent=intent)
        self._entries.append(entry)
        self._backend.add(query, embedding, intent)
        self._evict_if_needed()

    @property
    def size(self) -> int:
        """Number of entries in the cache."""
        return len(self._entries)

    def clear(self) -> None:
        """Remove all entries."""
        self._entries.clear()
        self._backend.clear()

    def close(self) -> None:
        """Release any underlying resources (e.g. DB connections)."""
        self._backend.close()

    # -- internal helpers --------------------------------------------------

    def _evict_if_needed(self) -> None:
        """Evict lowest-hit entries while size exceeds max_size."""
        while len(self._entries) > self.max_size:
            self._entries.sort(key=lambda e: e.hit_count)
            evicted = self._entries.pop(0)
            # Rebuild without the evicted entry
            self._backend.evict(self.max_size)

    # -- backward-compat shim for unit tests that inspect hit_count on _entries
