"""CLI entry point for coordination-patterns."""

from coordination_patterns.capability_router.pattern import AgentRouter, RoutingIntent
from coordination_patterns.llm_interface.config import LLMConfig


def main():
    print("=== Multi-Agent Coordination Patterns Demo ===\n")

    # 1. Show the router
    print("--- Capability Graph Router ---")
    router = AgentRouter()

    intent = RoutingIntent(
        action="find",
        resource="sales_report",
        parameters={"date_range": "Q1-2026"},
    )
    result = router.route_request(intent)
    print(f"Result: {result}\n")

    bad_intent = RoutingIntent(
        action="analyze",
        resource="server_log",
        parameters={},
    )
    result = router.route_request(bad_intent)
    print(f"Result: {result}\n")

    # 2. Show LLM config options
    print("--- LLM Interface Configs ---")
    print(f"Cacique: {LLMConfig.cacique().base_url}")
    print(f"Ollama:  {LLMConfig.ollama().base_url}")
    print(f"OpenAI:  {LLMConfig.openai().base_url}\n")

    # 3. Show intent extraction pipeline (dry run)
    print("--- Intent Extraction Pipeline ---")
    print("Full pipeline: Natural Language → LLM → RoutingIntent → AgentRouter")
    print("Usage:")
    print("  from coordination_patterns import IntentExtractor")
    print("  extractor = IntentExtractor(LLMConfig.cacique())")
    print("  result = extractor.process('Find the Q1 sales report')")
    print("  extractor.close()")


if __name__ == "__main__":
    main()
