"""Integration test configuration.

All tests here hit a real LLM endpoint (Ollama local) in deterministic mode
(seed=0, temperature=0) so outputs are reproducible — no flaky test failures.
"""

import pytest

from coordination_patterns.llm_interface.config import LLMConfig


def pytest_addoption(parser):
    parser.addoption(
        "--integ-host",
        default="localhost",
        help="Ollama host for integration tests (default: localhost)",
    )
    parser.addoption(
        "--integ-model",
        default="qwen3.5:0.8b",
        help="Model for integration tests (default: qwen3.5:0.8b)",
    )


@pytest.fixture(scope="session")
def integ_config(request):
    """Session-level LLMConfig for all integration tests — deterministic mode."""
    host = request.config.getoption("--integ-host")
    model = request.config.getoption("--integ-model")
    return LLMConfig.ollama(host=host, model=model, deterministic=True)


@pytest.fixture(scope="session")
def integ_extractor(integ_config):
    """Session-level IntentExtractor that shares one client across all tests."""
    from coordination_patterns.intent_extractor.extractor import IntentExtractor

    extractor = IntentExtractor(integ_config)
    yield extractor
    extractor.close()
