"""LLM configuration."""

from __future__ import annotations

import os
from dataclasses import dataclass, field


@dataclass
class LLMConfig:
    """Configuration for an LLM backend.

    Supports OpenAI-compatible endpoints (Cacique, OpenAI, OpenRouter, etc.).
    """

    # Provider type
    provider: str = "openai_compat"

    # OpenAI-compatible endpoint settings
    base_url: str = field(
        default_factory=lambda: os.getenv(
            "LLM_BASE_URL", "http://papia.tailde85bf.ts.net:8880/v1"
        )
    )
    model: str = field(
        default_factory=lambda: os.getenv("LLM_MODEL", "kokoro")
    )
    api_key: str = field(
        default_factory=lambda: os.getenv("LLM_API_KEY", "not-needed")
    )

    # Generation params
    temperature: float = 0.0
    max_tokens: int = 2048
    timeout: int = 30  # seconds

    # Convenience: pre-built configs for common setups
    @classmethod
    def cacique(cls) -> LLMConfig:
        """Cacique server (TTS + STT + LLM gateway)."""
        return cls(
            provider="openai_compat",
            base_url="http://papia.tailde85bf.ts.net:8880/v1",
            api_key="not-needed",
        )

    @classmethod
    def openai(cls, model: str = "gpt-4o") -> LLMConfig:
        """OpenAI API."""
        return cls(
            provider="openai_compat",
            base_url="https://api.openai.com/v1",
            model=model,
            api_key=os.getenv("OPENAI_API_KEY", ""),
        )

    @classmethod
    def ollama(cls, host: str = "localhost", model: str = "llama3.2") -> LLMConfig:
        """Local Ollama instance."""
        return cls(
            provider="openai_compat",
            base_url=f"http://{host}:11434/v1",
            model=model,
            api_key="not-needed",
        )
