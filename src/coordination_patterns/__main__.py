"""CLI entry point for coordination-patterns."""

from coordination_patterns.capability_router.pattern import AgentRouter, RoutingIntent


def main():
    router = AgentRouter()

    print("=== Capability Graph Router Demo ===\n")

    # Example: find a sales report
    intent = RoutingIntent(
        action="find",
        resource="sales_report",
        parameters={"date_range": "Q1-2026"},
    )
    result = router.route_request(intent)
    print(f"Result: {result}\n")

    # Example: unsupported combination
    bad_intent = RoutingIntent(
        action="analyze",
        resource="server_log",
        parameters={},
    )
    result = router.route_request(bad_intent)
    print(f"Result: {result}")


if __name__ == "__main__":
    main()
