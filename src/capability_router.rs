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
        Self { capability_graph }
    }

    pub fn route_request(&self, intent: &RoutingIntent) -> String {
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
            Some(agent) => self.dispatch_to_agent(agent, &intent.parameters),
            None => format!(
                "Error: No agent exists that can '{}' a '{}'.",
                serde_json::to_value(&intent.action)
                    .unwrap()
                    .as_str()
                    .unwrap(),
                serde_json::to_value(&intent.resource)
                    .unwrap()
                    .as_str()
                    .unwrap()
            ),
        }
    }

    fn dispatch_to_agent(&self, agent_name: &str, params: &Value) -> String {
        println!("Routing to {agent_name} with params: {params}");
        agent_name.to_string()
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}
