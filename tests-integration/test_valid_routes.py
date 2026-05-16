"""Integration tests: valid routes — LLM extracts intent → router dispatches to agent.

Smoke tests only. Each (action, resource) combo gets one prompt that the 0.8b model
reliably extracts. Not exhaustive — just prevents regressions.

These hit a real Ollama endpoint in deterministic mode. Run with:
    pytest tests-integration/test_valid_routes.py
"""

import pytest


class TestFindSalesReport:
    """action=find, resource=sales_report → SalesAgent"""

    def test_find(self, integ_extractor):
        result = integ_extractor.process("Find the Q1 sales report")
        assert result == "Success"


class TestAnalyzeSalesReport:
    """action=analyze, resource=sales_report → SalesAgent"""

    def test_analyze(self, integ_extractor):
        result = integ_extractor.process("I want to analyze our sales report performance")
        assert result == "Success"


class TestFindDocument:
    """action=find, resource=document → ComplianceAgent"""

    def test_find(self, integ_extractor):
        result = integ_extractor.process("I need to find the regulatory document")
        assert result == "Success"


class TestCreateServerLog:
    """action=create, resource=server_log → DevOpsAgent"""

    def test_create(self, integ_extractor):
        result = integ_extractor.process("Create a server log entry for the deployment")
        assert result == "Success"
