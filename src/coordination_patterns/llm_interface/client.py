"""OpenAI-compatible LLM client.

Talks to any OpenAI-compatible endpoint (Cacique, OpenAI, OpenRouter, Ollama, etc.).
"""

from __future__ import annotations

from typing import Any

import httpx

from coordination_patterns.llm_interface.config import LLMConfig


class LLMClient:
    """Minimal OpenAI-compatible chat client.

    Usage:
        client = LLMClient(LLMConfig.cacique())
        response = client.chat(messages=[...])
    """

    def __init__(self, config: LLMConfig | None = None):
        self.config = config or LLMConfig()
        self.client = httpx.Client(timeout=self.config.timeout)

    def chat(
        self,
        messages: list[dict[str, str]],
        system_prompt: str | None = None,
        response_format: dict | None = None,
        **extra: Any,
    ) -> str:
        """Send a chat completion and return the assistant text.

        Args:
            messages: List of {"role": ..., "content": ...} dicts.
            system_prompt: Optional system message prepended to messages.
            response_format: JSON schema for structured output (OpenAI format).
            **extra: Passed through to the request body.

        Returns:
            The assistant's text response.
        """
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

        resp = self.client.post(url, json=payload, headers=headers)
        resp.raise_for_status()
        data = resp.json()

        return data["choices"][0]["message"]["content"]

    def structured_chat(
        self,
        messages: list[dict[str, str]],
        schema: dict,
        system_prompt: str | None = None,
    ) -> dict:
        """Send a chat completion expecting structured JSON output.

        Uses OpenAI's response_format JSON schema feature.

        Args:
            messages: Chat messages.
            schema: JSON Schema for the expected response.
            system_prompt: Optional system message.

        Returns:
            Parsed JSON dict from the assistant response.
        """
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
        self.client.close()

    def __enter__(self) -> LLMClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()
