"""Integration tests: invalid routes — LLM extracts intent → router returns error.

Smoke tests for action+resource combos NOT in the capability graph.
The router should return an error string, not "Success".

These hit a real Ollama endpoint in deterministic mode. Run with:
    pytest tests-integration/test_invalid_routes.py
"""

import pytest


class TestFindServerLog:
    """action=find, resource=server_log → no agent"""

    def test_find(self, integ_extractor):
        result = integ_extractor.process("Find the server log from last night")
        assert "Error" in result


class TestCreateSalesReport:
    """action=create, resource=sales_report → no agent"""

    def test_create(self, integ_extractor):
        result = integ_extractor.process("Create a new sales report for Q1")
        assert "Error" in result


class TestCreateDocument:
    """action=create, resource=document → no agent"""

    def test_create(self, integ_extractor):
        result = integ_extractor.process("Create a document summarizing the audit")
        assert "Error" in result
