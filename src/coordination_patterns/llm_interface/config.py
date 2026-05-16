"""LLM and embedding configuration.

Two independent provider configs:
- LLMConfig     — chat completions (intent extraction)
- EmbeddingConfig — text embeddings (semantic cache)
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field


@dataclass
class LLMConfig:
    """Configuration for an LLM chat backend.

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
        default_factory=lambda: os.getenv("LLM_MODEL", "qwen3.5:2b")
    )
    api_key: str = field(
        default_factory=lambda: os.getenv("LLM_API_KEY", "not-needed")
    )

    # Generation params
    temperature: float = 0.0
    max_tokens: int = 2048
    timeout: int = 30  # seconds

    # Deterministic mode: lock seed + temperature for reproducible outputs.
    # When True, forces temperature=0.0 and sends seed=0 to the backend.
    # Intended for integration testing only — ensures the same input always
    # produces the same output, preventing flaky test failures.
    deterministic: bool = False

    @classmethod
    def ollama(
        cls,
        host: str = "localhost",
        model: str = "qwen3.5:2b",
        deterministic: bool = False,
    ) -> LLMConfig:
        """Local Ollama instance."""
        return cls(
            provider="ollama-local",
            base_url=f"http://{host}:11434/v1",
            model=model,
            api_key="not-needed",
            deterministic=deterministic,
        )

    def prepare(self) -> dict:
        """Build the base payload shared by chat/structured_chat.

        If deterministic mode is enabled, forces temperature=0.0 and
        attaches seed=0 so the backend produces identical output for
        the same input every time.
        """
        payload = {
            "model": self.model,
            "temperature": 0.0 if self.deterministic else self.temperature,
            "max_tokens": self.max_tokens,
        }
        if self.deterministic:
            payload["seed"] = 0
        return payload


@dataclass
class EmbeddingConfig:
    """Configuration for an embedding backend.

    Separate from LLMConfig — embeddings use a different model (e.g.,
    nomic-embed-text) and may hit a different endpoint.
    """

    # Provider type
    provider: str = "ollama-embedding"

    # Ollama native endpoint (not /v1)
    base_url: str = field(
        default_factory=lambda: os.getenv(
            "EMBEDDING_BASE_URL", "http://localhost:11434"
        )
    )
    model: str = field(
        default_factory=lambda: os.getenv("EMBEDDING_MODEL", "nomic-embed-text")
    )
    timeout: int = 30  # seconds

    @classmethod
    def ollama(
        cls,
        host: str = "localhost",
        model: str = "nomic-embed-text",
    ) -> EmbeddingConfig:
        """Local Ollama embedding instance."""
        return cls(
            provider="ollama-embedding",
            base_url=f"http://{host}:11434",
            model=model,
        )
