"""Tests for LLM interface."""

from coordination_patterns.llm_interface.config import LLMConfig


def test_config_defaults():
    config = LLMConfig()
    assert config.provider == "openai_compat"
    assert config.temperature == 0.0
    assert config.max_tokens == 2048


def test_config_ollama():
    config = LLMConfig.ollama(model="llama3.2")
    assert config.base_url == "http://localhost:11434/v1"
    assert config.model == "llama3.2"


def test_config_ollama_custom_host():
    config = LLMConfig.ollama(host="minadioro")
    assert config.base_url == "http://minadioro:11434/v1"
