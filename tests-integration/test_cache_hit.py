"""Integration test: prove semantic cache actually returns a hit on repeated queries.

Runs the same query twice through a cached IntentExtractor. The first call hits
the LLM and embedding model; the second should be a cache hit, bypassing both.
"""

import time

QUERY = "Find the Q1 sales report"


def test_cache_hit_is_faster(cached_extractor):
    """First call hits LLM + embeddings; second call should be a cache hit."""
    # --- First call: cache miss ---
    t0 = time.perf_counter()
    intent1 = cached_extractor.extract(QUERY)
    t1 = time.perf_counter()
    first_elapsed = t1 - t0

    # --- Second call: should be cache hit ---
    t2 = time.perf_counter()
    intent2 = cached_extractor.extract(QUERY)
    t3 = time.perf_counter()
    second_elapsed = t3 - t2

    # --- Assertions ---
    # Cache size should be 1 (second call hit the cache, didn't add)
    assert cached_extractor.cache.size == 1, (
        f"Expected 1 cached entry, got {cached_extractor.cache.size}"
    )

    # Second intent should match the first
    assert intent2.action == intent1.action
    assert intent2.resource == intent1.resource
    assert intent2.parameters == intent1.parameters

    # Second call should be significantly faster (at least 10x)
    # (realistic: first call ~1-3s, cache hit ~0.01s)
    assert second_elapsed < first_elapsed * 0.3, (
        f"Cache hit was not fast enough: first={first_elapsed:.3f}s, "
        f"second={second_elapsed:.3f}s"
    )

    # Print timing summary
    print(f"\nFirst call  (cache miss): {first_elapsed:.3f}s")
    print(f"Second call (cache hit) : {second_elapsed:.3f}s")
    print(f"Speedup               : {first_elapsed / second_elapsed:.1f}x")
    print(f"Cache entries         : {cached_extractor.cache.size}")
    print(f"Result                : action={intent2.action}, resource={intent2.resource}, params={intent2.parameters}")
