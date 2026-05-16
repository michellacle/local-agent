# Local Agent

**100% local AI agent coordination — no third-party APIs, no cloud dependencies.**

Local Agent provides proven patterns for coordinating multiple AI agents using
any locally hosted model that implements the OpenAI chat completions API.
Everything runs on your machine — intent extraction, semantic caching, agent
routing, and dispatch. No data ever leaves your network.

Built for privacy-focused use cases where your data stays yours.

**Current:** Python prototype using `uv`  
**Long-term goal:** Reimplement in Rust for performance and zero-cost abstractions

Proven patterns included: capability graphs, semantic intent extraction, agent routing, and caching.

## Quick Start

```bash
# Install all dependencies
uv sync --all-extras

# Extract intent (no cache)
uv run coordination-patterns extract "Find the Q1 sales report" --provider-llm ollama-local

# Extract intent with semantic cache
uv run coordination-patterns extract "Find the Q1 sales report" \
  --provider-llm ollama-local \
  --provider-embedding ollama-local \
  --cache

# Run tests
uv run pytest tests/ -v                      # Unit tests (fast, no network)
uv run pytest tests-integration/ -v          # Integration tests (hits real LLM)
uv run pytest tests-integration/ --ignore=tests-integration/test_cache_speed.py  # Skip cache tests (needs embed model)
```

## Two Providers

This system uses two independent providers:

- **`--provider-llm`** — chat model for intent extraction (e.g., `qwen3.5:0.8b`)
- **`--provider-embedding`** — embedding model for semantic cache (e.g., `nomic-embed-text`)

The embedding provider is only needed when `--cache` is enabled.

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

### 4. Semantic Intent Cache
Bypass the LLM for previously seen (or similar) queries. Requires a separate embedding model.

```python
from coordination_patterns import IntentExtractor, LLMConfig, EmbeddingConfig

with IntentExtractor(
    LLMConfig.ollama(model="qwen3.5:0.8b"),
    embed_config=EmbeddingConfig.ollama(model="nomic-embed-text"),
    cache_enabled=True,
) as extractor:
    # First call — hits LLM (~2s)
    extractor.process("Find the Q1 sales report")
    # Second call — cache hit (~0.01s)
    extractor.process("Find the Q1 sales report")
```

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                    User Input (Natural Language)                     │
│                 "Find the Q1 sales report"                           │
└───────────────────────────────┬──────────────────────────────────────┘
                                ▼
                     ┌──────────────────────┐
                     │   IntentExtractor    │
                     │   (Pattern #3)       │
                     └──────────┬───────────┘
                                │
                   ┌────────────┴────────────┐
                   │  Cache enabled?         │
                   └────────────┬────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              ▼                 │                 │
   ┌──────────────────┐   NO    │            YES  │
   │  EmbeddingClient │ ────────┼─────────────────┘
   │  (Pattern #4)    │         │
   │  + cosine_sim    │         │
   └───────┬──────────┘         │
           │                    │
           ▼                    │
┌─────────────────────┐         │
│  Embedding Model    │         │
│  nomic-embed-text   │         │
│  (ollama-local)     │         │
└────────┬────────────┘         │
         │ embedding vector     │
         ▼                      ▼
┌─────────────────────┐   ┌──────────────┐
│   SemanticCache     │   │  LLMClient   │ ◄── LLMConfig
│   (in-memory)       │   │ (Pattern #2) │     qwen3.5:0.8b
│                     │   └──────┬───────┘
│   Stores:           │          │
│   - query vector    │          │
│   - RoutingIntent   │          │
│   - hit count       │          │
└────────┬────────────┘          │
         │ cache HIT?            │
   ┌─────┴─────┐                 │
   │           │                 │
YES │       NO  │                 │
   ▼           │                 ▼
   │           │          ┌──────────────┐
   │           └────────> │  Agent       │
   │                      │   Router     │
   │                      │ (Pattern #1) │
   │                      └──────┬───────┘
   │                             │
   ▼                             ▼
   │                      ┌──────────────┐
   └────────────────────>│  Target      │
                         │  Agent       │
                         │ (SalesAgent, │
                         │  DevOpsAgent,│
                         │  etc.)       │
                         └──────────────┘

─── Two Independent Providers ──────────────────────────────────────────
  --provider-llm ollama-local        ──► LLMClient  ──► qwen3.5:0.8b
  --provider-embedding ollama-local  ──► EmbeddingClient ──► nomic-embed-text

## Project Structure

```
coordination-patterns/
├── pyproject.toml
├── src/coordination_patterns/
│   ├── __init__.py              # package exports
│   ├── __main__.py              # CLI
│   ├── capability_router/       # Pattern #1
│   │   └── pattern.py
│   ├── llm_interface/           # Pattern #2
│   │   ├── config.py            # LLMConfig + EmbeddingConfig
│   │   └── client.py            # LLMClient + EmbeddingClient
│   ├── intent_extractor/        # Pattern #3
│   │   └── extractor.py
│   └── semantic_cache/          # Pattern #4
│       ├── __init__.py          # SemanticCache + CachedEntry
│       └── utils.py             # cosine_similarity
├── tests/                       # Unit tests (no network)
└── tests-integration/           # Integration tests (hits real LLM)
    └── conftest.py              # Session-scoped fixtures
```

Built as a proper Python package using [uv](https://github.com/astral-sh/uv). Currently in Python as a prototype — the long-term goal is a Rust rewrite for performance and zero-cost abstractions.

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
