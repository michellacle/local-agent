"""Tests for semantic cache."""

from coordination_patterns.semantic_cache import SemanticCache, CachedEntry
from coordination_patterns.capability_router.pattern import RoutingIntent


def _intent(action="find", resource="sales_report") -> RoutingIntent:
    return RoutingIntent(action=action, resource=resource, parameters={})


def _embedding(*vals: float) -> list[float]:
    return list(vals)


def test_lookup_exact_match():
    cache = SemanticCache(threshold=0.92)
    emb = _embedding(1.0, 0.0, 0.0)
    cache.store("query", emb, _intent())
    result = cache.lookup(emb)
    assert result is not None
    assert result.action == "find"


def test_lookup_no_match():
    cache = SemanticCache(threshold=0.92)
    emb_a = _embedding(1.0, 0.0, 0.0)
    emb_b = _embedding(0.0, 1.0, 0.0)
    cache.store("query", emb_a, _intent())
    result = cache.lookup(emb_b)
    assert result is None


def test_lookup_similar_below_threshold():
    """Vectors with ~0.99 similarity fail a 0.995 threshold."""
    cache = SemanticCache(threshold=0.995)
    emb_a = _embedding(1.0, 0.1, 0.0)
    emb_b = _embedding(1.0, 0.0, 0.1)
    cache.store("query", emb_a, _intent())
    # Cosine similarity is ~0.99 which is below 0.995 threshold
    result = cache.lookup(emb_b)
    assert result is None


def test_lookup_hit_increments_hit_count():
    cache = SemanticCache(threshold=0.92)
    emb = _embedding(1.0, 0.0, 0.0)
    cache.store("query", emb, _intent())
    cache.lookup(emb)
    cache.lookup(emb)
    assert cache._entries[0].hit_count == 2


def test_eviction_keeps_most_hit():
    cache = SemanticCache(max_size=2)
    emb_a = _embedding(1.0, 0.0, 0.0)
    emb_b = _embedding(0.0, 1.0, 0.0)
    emb_c = _embedding(0.0, 0.0, 1.0)
    cache.store("a", emb_a, _intent())
    cache.store("b", emb_b, _intent(action="analyze"))
    # Hit b twice
    cache.lookup(emb_b)
    cache.lookup(emb_b)
    # Adding c should evict a (hit_count=0) but keep b (hit_count=2)
    cache.store("c", emb_c, _intent(action="create"))
    assert cache.size == 2
    remaining = [e.query for e in cache._entries]
    assert "b" in remaining
    assert "a" not in remaining


def test_clear():
    cache = SemanticCache()
    cache.store("q", _embedding(1.0), _intent())
    cache.clear()
    assert cache.size == 0


def test_cached_entry_serialization():
    entry = CachedEntry(
        query="test",
        embedding=[1.0, 2.0],
        intent=_intent(),
    )
    data = entry.model_dump()
    assert data["query"] == "test"
    assert data["embedding"] == [1.0, 2.0]
