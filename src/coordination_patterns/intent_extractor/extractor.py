"""Semantic Intent Extractor.

Uses an LLM to extract structured intents (action + resource + parameters)
from natural language user input, then feeds them to the AgentRouter.
"""

from __future__ import annotations

from coordination_patterns.capability_router.pattern import (
    ActionType,
    AgentRouter,
    ResourceType,
    RoutingIntent,
)
from coordination_patterns.llm_interface.client import LLMClient
from coordination_patterns.llm_interface.config import LLMConfig


# JSON Schema for intent extraction
INTENT_SCHEMA: dict = {
    "type": "object",
    "properties": {
        "action": {
            "type": "string",
            "enum": list(ActionType.__args__),  # type: ignore[attr-defined]
            "description": "The action to perform.",
        },
        "resource": {
            "type": "string",
            "enum": list(ResourceType.__args__),  # type: ignore[attr-defined]
            "description": "The resource to act on.",
        },
        "parameters": {
            "type": "object",
            "description": "Additional parameters extracted from the request.",
        },
    },
    "required": ["action", "resource"],
}

SYSTEM_PROMPT = """\
You are an intent extraction assistant.

Given a natural language request, extract:
- action: one of find, analyze, create
- resource: one of sales_report, server_log, document
- parameters: any relevant details as a dict

If the request doesn't match any action/resource combination,
still extract the closest action and resource you can infer.
"""


class IntentExtractor:
    """Extract structured intents from natural language using an LLM."""

    def __init__(self, config: LLMConfig | None = None):
        self.client = LLMClient(config)
        self.router = AgentRouter()

    def extract(self, user_input: str) -> RoutingIntent:
        """Extract a RoutingIntent from natural language.

        Args:
            user_input: The user's natural language request.

        Returns:
            A RoutingIntent with action, resource, and parameters.
        """
        messages = [
            {
                "role": "user",
                "content": f"Extract the intent from this request:\n\n{user_input}",
            }
        ]

        try:
            result = self.client.structured_chat(
                messages=messages,
                schema=INTENT_SCHEMA,
                system_prompt=SYSTEM_PROMPT,
            )
        except Exception as e:
            # Fallback: try parsing from plain text response
            print(f"Structured output failed ({e}), trying plain text fallback...")
            raw = self.client.chat(
                messages=messages,
                system_prompt=SYSTEM_PROMPT,
            )
            import json
            result = json.loads(raw)

        return RoutingIntent(
            action=result["action"],
            resource=result["resource"],
            parameters=result.get("parameters", {}),
        )

    def process(self, user_input: str) -> str:
        """Full pipeline: extract intent → route → dispatch.

        Args:
            user_input: The user's natural language request.

        Returns:
            The result of routing and dispatching the request.
        """
        intent = self.extract(user_input)
        print(f"Extracted intent: action={intent.action}, "
              f"resource={intent.resource}, params={intent.parameters}")
        return self.router.route_request(intent)

    def close(self) -> None:
        self.client.close()

    def __enter__(self) -> IntentExtractor:
        return self

    def __exit__(self, *args) -> None:
        self.close()
