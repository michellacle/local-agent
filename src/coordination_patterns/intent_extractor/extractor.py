"""Semantic Intent Extractor.

Uses an LLM to extract structured intents (action + resource + parameters)
from natural language user input, then feeds them to the AgentRouter.

Optionally uses a semantic cache to bypass the LLM for previously seen (or
similar) queries — reducing latency and cost.
"""

from __future__ import annotations

from coordination_patterns.capability_router.pattern import (
    ActionType,
    AgentRouter,
    ResourceType,
    RoutingIntent,
)
from coordination_patterns.llm_interface.client import EmbeddingClient, LLMClient
from coordination_patterns.llm_interface.config import EmbeddingConfig, LLMConfig
from coordination_patterns.semantic_cache import SemanticCache


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
    """Extract structured intents from natural language using an LLM.

    Optionally caches intents semantically — on repeat or similar queries,
    returns the cached intent immediately without calling the LLM.
    """

    def __init__(
        self,
        config: LLMConfig | None = None,
        embed_config: EmbeddingConfig | None = None,
        cache_enabled: bool = False,
    ):
        self.client = LLMClient(config)
        self.embed_client = EmbeddingClient(embed_config) if cache_enabled else None
        self.router = AgentRouter()
        self.cache_enabled = cache_enabled
        self.cache = SemanticCache() if cache_enabled else None

    def extract(self, user_input: str) -> RoutingIntent:
        """Extract a RoutingIntent from natural language.

        If caching is enabled, checks the cache before calling the LLM.
        Stores new extractions for future hits.

        Args:
            user_input: The user's natural language request.

        Returns:
            A RoutingIntent with action, resource, and parameters.
        """
        # Cache lookup — bypass LLM if we have a close match
        if self.cache_enabled and self.cache and self.embed_client:
            embedding = self.embed_client.embed(user_input)
            cached = self.cache.lookup(embedding)
            if cached is not None:
                print(f"Cache HIT (threshold {self.cache.threshold})")
                return cached

        # LLM extraction
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

        intent = RoutingIntent(
            action=result["action"],
            resource=result["resource"],
            parameters=result.get("parameters", {}),
        )

        # Store in cache for future hits
        if self.cache_enabled and self.cache and self.embed_client:
            embedding = self.embed_client.embed(user_input)
            self.cache.store(user_input, embedding, intent)
            print("Cache MISS → stored for future hits")

        return intent

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
        if self.embed_client:
            self.embed_client.close()

    def __enter__(self) -> IntentExtractor:
        return self

    def __exit__(self, *args) -> None:
        self.close()
