use local_agent::capability_router::{ActionType, AgentRouter, ResourceType, RoutingIntent};
use serde_json::json;

#[test]
fn test_route_known_action() {
    let router = AgentRouter::new();
    let intent = RoutingIntent {
        action: ActionType::Find,
        resource: ResourceType::SalesReport,
        parameters: json!({"date_range": "Q1-2026"}),
    };
    let result = router.route_request(&intent);
    assert_eq!(result, "SalesAgent");
}

#[test]
fn test_route_unknown_action() {
    let router = AgentRouter::new();
    let intent = RoutingIntent {
        action: ActionType::Analyze,
        resource: ResourceType::ServerLog,
        parameters: json!({}),
    };
    let result = router.route_request(&intent);
    assert!(result.contains("Error"));
}

#[test]
fn test_routing_intent_serialization() {
    let intent = RoutingIntent {
        action: ActionType::Find,
        resource: ResourceType::SalesReport,
        parameters: json!({"quarter": "Q1"}),
    };
    let serialized = serde_json::to_string(&intent).unwrap();
    let deserialized: RoutingIntent = serde_json::from_str(&serialized).unwrap();
    assert_eq!(intent.action, deserialized.action);
    assert_eq!(intent.resource, deserialized.resource);
    assert_eq!(intent.parameters, deserialized.parameters);
}
