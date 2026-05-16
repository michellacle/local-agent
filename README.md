# Multi-Agent Coordination Patterns

A collection of proven patterns for coordinating multiple AI agents — capability graphs, intent routing, consensus mechanisms, and more.

## Patterns

### 1. Capability Graph Router (`01_capability_router/`)
Route requests to specialized agents using a lookup table of `(action, resource)` pairs.

## Setup

```bash
cd ~/code/multi-agent-coordination-patterns
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## Adding New Patterns

1. Create a new numbered directory: `02_pattern_name/`
2. Implement the pattern in `pattern.py`
3. Add tests in `test_pattern.py`
4. Document in this README
