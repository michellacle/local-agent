"""Multi-agent coordination patterns."""

from __future__ import annotations

from coordination_patterns.capability_router.pattern import (
    AgentRouter,
    RoutingIntent,
)
from coordination_patterns.llm_interface.config import LLMConfig, EmbeddingConfig
from coordination_patterns.llm_interface.client import LLMClient, EmbeddingClient
from coordination_patterns.intent_extractor.extractor import IntentExtractor
from coordination_patterns.semantic_cache import (
    CachedEntry,
    SemanticCache,
)
from coordination_patterns.semantic_cache.store import (
    CacheStoreProtocol,
    MemoryCacheStore,
    SqliteCacheStore,
)

__all__: list[str] = [
    "AgentRouter",
    "RoutingIntent",
    "LLMConfig",
    "LLMClient",
    "IntentExtractor",
    "SemanticCache",
    "CachedEntry",
    "EmbeddingConfig",
    "EmbeddingClient",
    "CacheStoreProtocol",
    "MemoryCacheStore",
    "SqliteCacheStore",
]
__version__: str = "0.1.0"
