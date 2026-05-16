"""Integration test for SQLite-backed semantic cache persistence.

Verifies that cached entries survive process restarts.
"""

import os
import tempfile

from coordination_patterns.capability_router.pattern import RoutingIntent
from coordination_patterns.semantic_cache import SemanticCache


def test_sqlite_persistence_survives_restart() -> None:
    """Entries stored with SQLite backend persist across cache instances."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = os.path.join(tmpdir, "cache.db")
        embedding = [0.1, 0.2, 0.3, 0.4, 0.5]
        intent = RoutingIntent(action="find", resource="sales_report", parameters={"quarter": "Q1"})

        # First instance: store an entry
        cache1 = SemanticCache(store="sqlite", store_path=db_path)
        cache1.store("Find the Q1 sales report", embedding, intent)
        assert cache1.size == 1
        cache1.close()

        # Second instance: reload from disk
        cache2 = SemanticCache(store="sqlite", store_path=db_path)
        cached = cache2.lookup(embedding)
        assert cached is not None
        assert cached.action == "find"
        assert cached.resource == "sales_report"
        assert cache2.size == 1
        cache2.close()


def test_sqlite_clear_removes_entries() -> None:
    """Clearing the cache removes all entries from disk."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = os.path.join(tmpdir, "cache.db")
        embedding = [0.5, 0.5, 0.5]
        intent = RoutingIntent(action="create", resource="document", parameters={})

        cache = SemanticCache(store="sqlite", store_path=db_path)
        cache.store("Create a new document", embedding, intent)
        cache.clear()
        assert cache.size == 0
        result = cache.lookup(embedding)
        assert result is None
        cache.close()
