"""LLM configuration."""

from __future__ import annotations

import os
from dataclasses import dataclass, field


@dataclass
class LLMConfig:
    """Configuration for an LLM backend.

    Targets local Ollama instances via the OpenAI-compatible API.
    """

    # Provider type
    provider: str = "openai_compat"

    # OpenAI-compatible endpoint settings
    base_url: str = field(
        default_factory=lambda: os.getenv(
            "LLM_BASE_URL", "http://localhost:11434/v1"
        )
    )
    model: str = field(
        default_factory=lambda: os.getenv("LLM_MODEL", "llama3.2")
    )
    api_key: str = field(
        default_factory=lambda: os.getenv("LLM_API_KEY", "not-needed")
    )

    # Generation params
    temperature: float = 0.0
    max_tokens: int = 2048
    timeout: int = 30  # seconds

    @classmethod
    def ollama(cls, host: str = "localhost", model: str = "llama3.2") -> LLMConfig:
        """Local Ollama instance."""
        return cls(
            provider="openai_compat",
            base_url=f"http://{host}:11434/v1",
            model=model,
            api_key="not-needed",
        )
