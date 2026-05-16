"""Tests for intent extractor."""

from coordination_patterns.intent_extractor.extractor import INTENT_SCHEMA, SYSTEM_PROMPT


def test_intent_schema_has_required_fields():
    assert "type" in INTENT_SCHEMA
    assert INTENT_SCHEMA["type"] == "object"
    assert "properties" in INTENT_SCHEMA
    assert "action" in INTENT_SCHEMA["properties"]
    assert "resource" in INTENT_SCHEMA["properties"]
    assert "action" in INTENT_SCHEMA["required"]
    assert "resource" in INTENT_SCHEMA["required"]


def test_system_prompt_exists():
    assert SYSTEM_PROMPT
    assert "intent" in SYSTEM_PROMPT.lower()
