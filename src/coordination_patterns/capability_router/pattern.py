"""
Capability Graph Router

Route requests to specialized agents using a lookup table of (action, resource) pairs.

In a production environment, an LLM would extract structured intents from
natural language input before passing them to this routing logic.
"""

from pydantic import BaseModel
from typing import Literal


# 1. Define the "Vocabulary" of the system
ActionType = Literal["find", "analyze", "create"]
ResourceType = Literal["sales_report", "server_log", "document"]


class RoutingIntent(BaseModel):
    action: ActionType
    resource: ResourceType
    parameters: dict


class AgentRouter:
    """Route requests to specialized agents via a capability graph."""

    def __init__(self):
        # The Capability Graph: Maps (Action, Resource) -> Agent Name
        self.capability_graph: dict[tuple[str, str], str] = {
            ("find", "sales_report"): "SalesAgent",
            ("analyze", "sales_report"): "SalesAgent",
            ("find", "document"): "ComplianceAgent",
            ("create", "server_log"): "DevOpsAgent",
        }

    def route_request(self, intent: RoutingIntent) -> str:
        key = (intent.action, intent.resource)
        target_agent_name = self.capability_graph.get(key)

        if not target_agent_name:
            return (
                f"Error: No agent exists that can '{intent.action}' "
                f"a '{intent.resource}'."
            )

        return self.dispatch_to_agent(target_agent_name, intent.parameters)

    def dispatch_to_agent(self, agent_name: str, params: dict) -> str:
        print(f"Routing to {agent_name} with params: {params}")
        return agent_name
