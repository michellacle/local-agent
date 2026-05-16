# Multi-Agent Coordination Patterns

Proven patterns for coordinating multiple AI agents — capability graphs, intent routing, consensus mechanisms, and more.

Built as a proper Python package using [uv](https://github.com/astral-sh/uv).

## Quick Start

```bash
# Install the package in dev mode
uv sync --all-extras

# Run the demo
uv run coordination-patterns

# Run tests
uv run pytest
```

## Patterns

### 1. Capability Graph Router
Route requests to specialized agents using a lookup table of `(action, resource)` pairs.

```python
from coordination_patterns import AgentRouter, RoutingIntent

router = AgentRouter()
intent = RoutingIntent(
    action="find",
    resource="sales_report",
    parameters={"date_range": "Q1-2026"},
)
result = router.route_request(intent)
# → "Routing to SalesAgent with params: {'date_range': 'Q1-2026'}"
# → "Success"
```

## Project Structure

```
coordination-patterns/
├── pyproject.toml
├── uv.lock
├── src/
│   └── coordination_patterns/
│       ├── __init__.py
│       ├── __main__.py
│       └── capability_router/
│           ├── __init__.py
│           └── pattern.py
└── tests/
    └── test_capability_router.py
```

## uv Cheat Sheet (Learning Guide)

| Command | What it does |
|---|---|
| `uv init` | Create a new project |
| `uv add <pkg>` | Add dependency + update lockfile |
| `uv sync` | Install deps from lockfile (like `pip install -e .`) |
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
