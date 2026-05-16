"""Multi-agent coordination patterns."""

from coordination_patterns.capability_router.pattern import (
    AgentRouter,
    RoutingIntent,
)
from coordination_patterns.llm_interface.config import LLMConfig, EmbeddingConfig
from coordination_patterns.llm_interface.client import LLMClient, EmbeddingClient
from coordination_patterns.intent_extractor.extractor import IntentExtractor
from coordination_patterns.semantic_cache import (
    SemanticCache,
    CachedEntry,
)

__all__ = [
    "AgentRouter",
    "RoutingIntent",
    "LLMConfig",
    "LLMClient",
    "IntentExtractor",
    "SemanticCache",
    "CachedEntry",
    "EmbeddingConfig",
    "EmbeddingClient",
]
__version__ = "0.1.0"
