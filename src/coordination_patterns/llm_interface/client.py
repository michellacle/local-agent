"""OpenAI-compatible LLM client and embedding client.

Talks to any OpenAI-compatible endpoint (Cacique, OpenAI, OpenRouter, Ollama, etc.).

Two independent clients:
- LLMClient       — chat completions (intent extraction)
- EmbeddingClient — text embeddings (semantic cache)
"""

from __future__ import annotations

from typing import Any

import httpx

from coordination_patterns.llm_interface.config import EmbeddingConfig, LLMConfig


class LLMClient:
    """Minimal OpenAI-compatible chat client.

    Usage:
        client = LLMClient(LLMConfig.ollama())
        response = client.chat(messages=[...])
    """

    def __init__(self, config: LLMConfig | None = None):
        self.config = config or LLMConfig()
        self._http = httpx.Client(timeout=self.config.timeout)

    def chat(
        self,
        messages: list[dict[str, str]],
        system_prompt: str | None = None,
        response_format: dict | None = None,
        **extra: Any,
    ) -> str:
        """Send a chat completion and return the assistant text."""
        msg_list = list(messages)
        if system_prompt:
            msg_list.insert(0, {"role": "system", "content": system_prompt})

        payload: dict[str, Any] = self.config.prepare()
        payload["messages"] = msg_list

        if response_format:
            payload["response_format"] = response_format

        if extra:
            payload.update(extra)

        url = f"{self.config.base_url}/chat/completions"
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.config.api_key}",
        }

        resp = self._http.post(url, json=payload, headers=headers)
        resp.raise_for_status()
        data = resp.json()

        return data["choices"][0]["message"]["content"]

    def structured_chat(
        self,
        messages: list[dict[str, str]],
        schema: dict,
        system_prompt: str | None = None,
    ) -> dict:
        """Send a chat completion expecting structured JSON output."""
        import json

        response_format = {
            "type": "json_schema",
            "json_schema": {
                "name": "structured_output",
                "schema": schema,
                "strict": True,
            },
        }

        raw = self.chat(
            messages=messages,
            system_prompt=system_prompt,
            response_format=response_format,
        )

        return json.loads(raw)

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> LLMClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()


class EmbeddingClient:
    """Client for text embeddings via Ollama's native /api/embeddings endpoint.

    Separate from LLMClient — uses its own config and HTTP session.

    Usage:
        client = EmbeddingClient(EmbeddingConfig.ollama())
        vector = client.embed("hello world")
    """

    def __init__(self, config: EmbeddingConfig | None = None):
        self.config = config or EmbeddingConfig()
        self._http = httpx.Client(timeout=self.config.timeout)

    def embed(self, text: str) -> list[float]:
        """Compute an embedding vector for the given text."""
        url = f"{self.config.base_url}/api/embeddings"
        payload = {
            "model": self.config.model,
            "input": text,
        }
        headers = {"Content-Type": "application/json"}

        resp = self._http.post(url, json=payload, headers=headers)
        resp.raise_for_status()
        data = resp.json()
        # Ollama native endpoint returns {"embedding": [...]}
        return data["embedding"]

    def close(self) -> None:
        self._http.close()

    def __enter__(self) -> EmbeddingClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()
