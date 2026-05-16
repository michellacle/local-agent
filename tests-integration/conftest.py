"""Integration test configuration.

All tests here hit a real LLM endpoint (Ollama local) in deterministic mode
(seed=0, temperature=0) so outputs are reproducible — no flaky test failures.
"""

import pytest

from coordination_patterns.llm_interface.config import EmbeddingConfig, LLMConfig


def pytest_addoption(parser):
    parser.addoption(
        "--integ-host",
        default="localhost",
        help="Ollama host for integration tests (default: localhost)",
    )
    parser.addoption(
        "--integ-model",
        default="qwen3.5:0.8b",
        help="LLM model for integration tests (default: qwen3.5:0.8b)",
    )
    parser.addoption(
        "--integ-embed-model",
        default="nomic-embed-text",
        help="Embedding model for integration tests (default: nomic-embed-text)",
    )


@pytest.fixture(scope="session")
def integ_config(request):
    """Session-level LLMConfig for all integration tests — deterministic mode."""
    host = request.config.getoption("--integ-host")
    model = request.config.getoption("--integ-model")
    return LLMConfig.ollama(host=host, model=model, deterministic=True)


@pytest.fixture(scope="session")
def integ_embed_config(request):
    """Session-level EmbeddingConfig for cached integration tests."""
    host = request.config.getoption("--integ-host")
    model = request.config.getoption("--integ-embed-model")
    return EmbeddingConfig.ollama(host=host, model=model)


@pytest.fixture(scope="session")
def integ_extractor(integ_config):
    """Session-level IntentExtractor without cache."""
    from coordination_patterns.intent_extractor.extractor import IntentExtractor

    extractor = IntentExtractor(integ_config)
    yield extractor
    extractor.close()


@pytest.fixture(scope="session")
def cached_extractor(integ_config, integ_embed_config):
    """Session-level IntentExtractor with semantic cache enabled."""
    from coordination_patterns.intent_extractor.extractor import IntentExtractor

    extractor = IntentExtractor(
        integ_config, embed_config=integ_embed_config, cache_enabled=True
    )
    yield extractor
    extractor.close()
