"""Integration tests: invalid routes — LLM extracts intent → router returns error.

These test all action+resource combos that are NOT in the capability graph.
The router should return an error string, not "Success".

There are 9 total combos (3 actions x 3 resources). 4 are valid (tested in
test_valid_routes.py). The remaining 5 should fail routing.

Run with:
    pytest tests-integration/test_invalid_routes.py
"""

import pytest


# ─── find + server_log → ERROR ────────────────────────────────────────────────


class TestFindServerLog:
    """action=find, resource=server_log → no agent"""

    def test_find_server_log(self, integ_extractor):
        result = integ_extractor.process("Find the server log from last night")
        assert "Error" in result


# ─── analyze + server_log → ERROR ─────────────────────────────────────────────


class TestAnalyzeServerLog:
    """action=analyze, resource=server_log → no agent"""

    def test_analyze_server_log(self, integ_extractor):
        result = integ_extractor.process("Analyze the server log for errors")
        assert "Error" in result


# ─── analyze + document → ERROR ───────────────────────────────────────────────


class TestAnalyzeDocument:
    """action=analyze, resource=document → no agent"""

    def test_analyze_document(self, integ_extractor):
        result = integ_extractor.process("Analyze the document for key findings")
        assert "Error" in result


# ─── create + sales_report → ERROR ────────────────────────────────────────────


class TestCreateSalesReport:
    """action=create, resource=sales_report → no agent"""

    def test_create_sales_report(self, integ_extractor):
        result = integ_extractor.process("Create a new sales report for Q1")
        assert "Error" in result


# ─── create + document → ERROR ────────────────────────────────────────────────


class TestCreateDocument:
    """action=create, resource=document → no agent"""

    def test_create_document(self, integ_extractor):
        result = integ_extractor.process("Create a document summarizing the audit")
        assert "Error" in result
