use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The kind of operation an agent can perform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Locate an existing resource.
    Find,
    /// Perform analysis on a resource.
    Analyze,
    /// Generate a new resource.
    Create,
}

/// The kind of domain resource an action operates on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// A sales report containing transactional data.
    SalesReport,
    /// A server log with operational events.
    ServerLog,
    /// A general-purpose document.
    Document,
    /// A loan application for approval workflow.
    LoanApplication,
}

/// A structured intent extracted from natural language, used to route to the correct agent.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RoutingIntent {
    /// The action to perform.
    pub action: ActionType,
    /// The resource type the action targets.
    pub resource: ResourceType,
    /// Arbitrary key-value parameters extracted from the request.
    #[serde(default)]
    pub parameters: Value,
}

/// Result of routing an intent to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RouteResult {
    /// The intent was matched to a registered agent.
    Routed {
        /// The name of the matched agent.
        agent_name: String,
        /// The parameters to pass to the agent.
        parameters: Value,
    },
    /// No registered agent can handle the given action/resource pair.
    NotFound {
        /// The action that was requested.
        action: String,
        /// The resource that was requested.
        resource: String,
    },
}

impl RouteResult {
    /// Returns true if the intent was successfully routed to an agent.
    pub fn is_routed(&self) -> bool {
        matches!(self, RouteResult::Routed { .. })
    }

    /// Returns the agent name if routed, or None if not found.
    pub fn agent_name(&self) -> Option<&str> {
        match self {
            RouteResult::Routed { agent_name, .. } => Some(agent_name),
            RouteResult::NotFound { .. } => None,
        }
    }

    /// Returns a human-readable description of the routing result.
    pub fn display(&self) -> String {
        match self {
            RouteResult::Routed { agent_name, .. } => format!("Routed to {agent_name}"),
            RouteResult::NotFound { action, resource } => {
                format!("No agent exists that can '{action}' a '{resource}'")
            }
        }
    }
}

/// Maps (action, resource) pairs to agent names and dispatches requests.
pub struct AgentRouter {
    /// Internal lookup table from (action, resource) to agent name.
    capability_graph: std::collections::HashMap<(String, String), String>,
}

impl AgentRouter {
    pub fn new() -> Self {
        let mut capability_graph = std::collections::HashMap::new();
        capability_graph.insert(("find".into(), "sales_report".into()), "SalesAgent".into());
        capability_graph.insert(
            ("analyze".into(), "sales_report".into()),
            "SalesAgent".into(),
        );
        capability_graph.insert(("find".into(), "document".into()), "ComplianceAgent".into());
        capability_graph.insert(("create".into(), "server_log".into()), "DevOpsAgent".into());
        capability_graph.insert(
            ("create".into(), "loan_application".into()),
            "LoanOrchestratorAgent".into(),
        );
        Self { capability_graph }
    }

    pub fn route_request(&self, intent: &RoutingIntent) -> RouteResult {
        let key = (
            serde_json::to_value(&intent.action)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            serde_json::to_value(&intent.resource)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
        );

        match self.capability_graph.get(&key) {
            Some(agent) => {
                println!("Routing to {} with params: {}", agent, intent.parameters);
                RouteResult::Routed {
                    agent_name: agent.clone(),
                    parameters: intent.parameters.clone(),
                }
            }
            None => RouteResult::NotFound {
                action: serde_json::to_value(&intent.action)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
                resource: serde_json::to_value(&intent.resource)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
            },
        }
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(
            result,
            RouteResult::Routed {
                agent_name: "SalesAgent".into(),
                parameters: json!({"date_range": "Q1-2026"}),
            }
        );
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
        assert!(matches!(result, RouteResult::NotFound { .. }));
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
}
