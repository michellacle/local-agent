"""Tests for capability graph router."""

from coordination_patterns.capability_router.pattern import AgentRouter, RoutingIntent


def test_route_known_action():
    router = AgentRouter()
    intent = RoutingIntent(
        action="find",
        resource="sales_report",
        parameters={"date_range": "Q1-2026"},
    )
    result = router.route_request(intent)
    assert result == "SalesAgent"


def test_route_unknown_action():
    router = AgentRouter()
    intent = RoutingIntent(
        action="analyze",
        resource="server_log",
        parameters={},
    )
    result = router.route_request(intent)
    assert "Error" in result


def test_routing_intent_validation():
    """RoutingIntent rejects invalid actions/resources."""
    import pytest
    from pydantic import ValidationError

    with pytest.raises(ValidationError):
        RoutingIntent(
            action="delete",  # not a valid ActionType
            resource="sales_report",
            parameters={},
        )
