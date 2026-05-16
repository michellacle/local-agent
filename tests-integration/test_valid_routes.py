"""Integration tests: valid routes — LLM extracts intent → router dispatches to agent.

Each test sends a natural language prompt, extracts the intent via the real LLM,
and verifies the router successfully routes it to the expected agent.

These hit a real Ollama endpoint. Run with:
    pytest tests-integration/test_valid_routes.py
"""

import pytest
from coordination_patterns.capability_router.pattern import AgentRouter


# ─── find + sales_report → SalesAgent ─────────────────────────────────────────


class TestFindSalesReport:
    """action=find, resource=sales_report → SalesAgent"""

    def test_direct_request(self, integ_extractor):
        result = integ_extractor.process("Find the Q1 sales report")
        assert result == "Success"

    def test_with_timeframe(self, integ_extractor):
        result = integ_extractor.process("I need to find the sales report for last month")
        assert result == "Success"

    def test_find_latest(self, integ_extractor):
        result = integ_extractor.process("Can you find the latest sales report?")
        assert result == "Success"


# ─── analyze + sales_report → SalesAgent ──────────────────────────────────────


class TestAnalyzeSalesReport:
    """action=analyze, resource=sales_report → SalesAgent"""

    def test_analyze_trends(self, integ_extractor):
        result = integ_extractor.process("Analyze the sales report trends")
        assert result == "Success"

    def test_review_report(self, integ_extractor):
        result = integ_extractor.process("I want to analyze our sales report performance")
        assert result == "Success"

    def test_breakdown(self, integ_extractor):
        result = integ_extractor.process("Can you analyze the sales report by region?")
        assert result == "Success"


# ─── find + document → ComplianceAgent ────────────────────────────────────────


class TestFindDocument:
    """action=find, resource=document → ComplianceAgent"""

    def test_find_policy(self, integ_extractor):
        result = integ_extractor.process("Find the compliance document for data privacy")
        assert result == "Success"

    def test_locate_document(self, integ_extractor):
        result = integ_extractor.process("I need to find the regulatory document")
        assert result == "Success"

    def test_search_document(self, integ_extractor):
        result = integ_extractor.process("Search for the document about safety guidelines")
        assert result == "Success"


# ─── create + server_log → DevOpsAgent ────────────────────────────────────────


class TestCreateServerLog:
    """action=create, resource=server_log → DevOpsAgent"""

    def test_create_log(self, integ_extractor):
        result = integ_extractor.process("Create a server log entry for the deployment")
        assert result == "Success"

    def test_new_log_entry(self, integ_extractor):
        result = integ_extractor.process("I need to create a new server log for the incident")
        assert result == "Success"

    def test_generate_log(self, integ_extractor):
        result = integ_extractor.process("Generate a server log for the maintenance window")
        assert result == "Success"
