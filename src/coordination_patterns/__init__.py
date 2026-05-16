"""Multi-agent coordination patterns."""

from coordination_patterns.capability_router.pattern import (
    AgentRouter,
    RoutingIntent,
)
from coordination_patterns.llm_interface.config import LLMConfig
from coordination_patterns.llm_interface.client import LLMClient
from coordination_patterns.intent_extractor.extractor import IntentExtractor

__all__ = [
    "AgentRouter",
    "RoutingIntent",
    "LLMConfig",
    "LLMClient",
    "IntentExtractor",
]
__version__ = "0.1.0"
