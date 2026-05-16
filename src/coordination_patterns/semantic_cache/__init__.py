"""Semantic Intent Cache.

Before extracting intent via LLM, check a local cache of previously seen
requests. If a sufficiently similar query exists, return the cached intent
immediately — bypassing the LLM entirely. Every successful extraction self-populates
the cache for future hits.
"""

from __future__ import annotations

import time
from pydantic import BaseModel

from coordination_patterns.capability_router.pattern import RoutingIntent
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
    """In-memory semantic cache for routing intents.

    Stores (query, embedding, intent) tuples. On lookup, computes cosine
    similarity between the query embedding and all stored embeddings. Returns
    the cached intent if the best match exceeds the threshold.
    """

    def __init__(self, threshold: float = 0.92, max_size: int = 1000):
        """
        Args:
            threshold: Minimum cosine similarity to count as a cache hit.
            max_size: Maximum number of entries before LRU eviction.
        """
        self.threshold = threshold
        self.max_size = max_size
        self._entries: list[CachedEntry] = []

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
            return best_entry.intent

        return None

    def store(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        """Add an entry. Evict least-recently-hit if at max_size."""
        self._entries.append(CachedEntry(query=query, embedding=embedding, intent=intent))
        if len(self._entries) > self.max_size:
            self._entries.sort(key=lambda e: e.hit_count)
            self._entries = self._entries[-self.max_size:]

    @property
    def size(self) -> int:
        """Number of entries in the cache."""
        return len(self._entries)

    def clear(self) -> None:
        """Remove all entries."""
        self._entries.clear()
