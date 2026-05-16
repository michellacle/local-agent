"""
Capability Graph Router

Route requests to specialized agents using a lookup table of (action, resource) pairs.
Inspired by multi-agent coordination patterns (Ch. 5, "Designing Agentic AI Systems").

In a production environment, an LLM would extract structured intents from
natural language input before passing them to this routing logic.
"""

from pydantic import BaseModel
from typing import Literal, List, Tuple


# 1. Define the "Vocabulary" of the system
ActionType = Literal["find", "analyze", "create"]
ResourceType = Literal["sales_report", "server_log", "document"]


class RoutingIntent(BaseModel):
    action: ActionType
    resource: ResourceType
    parameters: dict


class AgentRouter:
    def __init__(self):
        # The Capability Graph: Maps (Action, Resource) -> Agent Name
        self.capability_graph = {
            ("find", "sales_report"): "SalesAgent",
            ("analyze", "sales_report"): "SalesAgent",
            ("find", "document"): "ComplianceAgent",
            ("create", "server_log"): "DevOpsAgent",
        }

    def route_request(self, intent: RoutingIntent):
        # 1. Lookup the capability in the graph
        key = (intent.action, intent.resource)

        target_agent_name = self.capability_graph.get(key)

        # 2. Safety Check: If no link exists in the graph, block the request
        if not target_agent_name:
            return (
                f"Error: No agent exists that can '{intent.action}' "
                f"a '{intent.resource}'."
            )

        # 3. Dispatch (Simplified)
        return self.dispatch_to_agent(target_agent_name, intent.parameters)

    def dispatch_to_agent(self, agent_name: str, params: dict):
        print(f"Routing to {agent_name} with params: {params}")
        # In real code, this would instantiate the agent class and call .run()
        return "Success"


if __name__ == "__main__":
    router = AgentRouter()

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
