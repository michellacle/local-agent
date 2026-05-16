# Multi-Agent Coordination Patterns

Proven patterns for coordinating multiple AI agents — capability graphs, intent routing, semantic extraction, and more.

Built as a proper Python package using [uv](https://github.com/astral-sh/uv).

## Quick Start

```bash
# Install all dependencies
uv sync --all-extras

# Run the demo
uv run coordination-patterns

# Run tests
uv run pytest tests/ -v              # Unit tests (fast, no network)
uv run pytest tests-integration/ -v  # Integration tests (hits real LLM)
```

## Patterns

### 1. Capability Graph Router
Route requests to specialized agents using `(action, resource)` → `agent` lookup.

```python
from coordination_patterns import AgentRouter, RoutingIntent

router = AgentRouter()
intent = RoutingIntent(action="find", resource="sales_report", parameters={"quarter": "Q1"})
result = router.route_request(intent)
# → "Routing to SalesAgent with params: {'quarter': 'Q1'}"
```

### 2. LLM Interface (Swappable Backends)
Talk to any OpenAI-compatible LLM without hardcoding endpoints.

```python
from coordination_patterns import LLMClient, LLMConfig

# Swap backends by changing config:
client = LLMClient(LLMConfig.ollama())     # Local Ollama

response = client.chat(messages=[{"role": "user", "content": "Hello"}])
```

### 3. Semantic Intent Extraction (Full Pipeline)
Natural language → LLM extracts structured intent → route to agent.

```python
from coordination_patterns import IntentExtractor, LLMConfig

with IntentExtractor(LLMConfig.ollama()) as extractor:
    result = extractor.process("Find the Q1 sales report")
    # Internally:
    # 1. LLM extracts: {action: "find", resource: "sales_report", parameters: {"quarter": "Q1"}}
    # 2. Router dispatches to SalesAgent
    # 3. Returns result
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   User Input (NL)                       │
│            "Find the Q1 sales report"                   │
└──────────────────────┬──────────────────────────────────┘
                       ▼
            ┌────────────────────┐
            │  IntentExtractor   │
            │  (Pattern #3)      │
            └───────┬────────────┘
                    │ extracts structured intent
                    ▼
            ┌────────────────────┐
            │    LLMClient       │ ◄── LLMConfig (swappable)
            │  (Pattern #2)      │     cacique / ollama / openai
            └───────┬────────────┘
                    │ returns RoutingIntent
                    ▼
            ┌────────────────────┐
            │    AgentRouter     │
            │  (Pattern #1)      │
            └───────┬────────────┘
                    │ capability_graph lookup
                    ▼
            ┌────────────────────┐
            │   Target Agent     │
            │  SalesAgent, etc.  │
            └────────────────────┘
```

## Project Structure

```
coordination-patterns/
├── pyproject.toml
├── uv.lock
├── src/coordination_patterns/
│   ├── __init__.py              # package exports
│   ├── __main__.py              # CLI demo
│   ├── capability_router/       # Pattern #1
│   │   └── pattern.py
│   ├── llm_interface/           # Pattern #2
│   │   ├── config.py            # LLMConfig (swappable backends)
│   │   └── client.py            # LLMClient (OpenAI-compatible)
│   └── intent_extractor/        # Pattern #3
│       └── extractor.py         # IntentExtractor (NL → route)
├── tests/                       # Unit tests (no network)
└── tests-integration/           # Integration tests (hits real LLM)
    └── conftest.py              # Session-scoped fixtures, deterministic mode
```

## uv Quick Reference

| Command | What it does |
|---|---|
| `uv init` | Create a new project |
| `uv add <pkg>` | Add dependency + update lockfile |
| `uv sync` | Install deps from lockfile |
| `uv run <cmd>` | Run a command in the virtual env |
| `uv remove <pkg>` | Remove a dependency |
| `uv lock` | Regenerate the lockfile |
| `uv build` | Build a wheel/sdist |
| `uv publish` | Publish to PyPI |

## Adding New Patterns

1. Create a new module under `src/coordination_patterns/<name>/`
2. Add `__init__.py` + `pattern.py`
3. Add tests under `tests/`
4. Export from `src/coordination_patterns/__init__.py`
