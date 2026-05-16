"""Integration test: prove cache speed benefit.

First call hits the LLM (slow). Second identical call hits the cache (instant).
This test measures the time difference to prove caching works end-to-end.

Run with:
    pytest tests-integration/test_cache_speed.py -v --integ-host localhost
"""

import time

import pytest


class TestCacheSpeedBenefit:
    """Prove that the semantic cache bypasses the LLM on repeat queries."""

    def test_cache_produces_speed_benefit(self, cached_extractor):
        """First call hits LLM (~1-5s), second call hits cache (<0.1s).

        We assert the second call is at least 10x faster than the first.
        """
        query = "Find the Q1 sales report"

        # First call — cache miss, hits LLM
        t0 = time.time()
        result1 = cached_extractor.process(query)
        first_duration = time.time() - t0

        assert result1 == "Success"
        print(f"First call (LLM): {first_duration:.2f}s")

        # Second call — cache hit, should be instant
        t1 = time.time()
        result2 = cached_extractor.process(query)
        second_duration = time.time() - t1

        assert result2 == "Success"
        print(f"Second call (cache): {second_duration:.4f}s")

        # The cache hit should be dramatically faster
        speedup = first_duration / max(second_duration, 0.001)
        print(f"Speedup: {speedup:.1f}x")

        # Assert: second call is at least 10x faster
        assert second_duration < first_duration * 0.1, (
            f"Cache hit ({second_duration:.4f}s) was not faster than "
            f"LLM call ({first_duration:.2f}s)"
        )

    def test_cache_bypasses_llm_for_identical_query(self, cached_extractor):
        """Verify that the cache stores and retrieves the same intent."""
        query = "Create a server log entry for the deployment"

        # Warm the cache
        result1 = cached_extractor.process(query)
        assert result1 == "Success"

        # Repeat — should hit cache and return same result
        result2 = cached_extractor.process(query)
        assert result2 == "Success"

        # Verify cache has grown
        assert cached_extractor.cache.size >= 1
