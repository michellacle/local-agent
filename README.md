# Local Agent

**100% local AI agent coordination — no third-party APIs, no cloud dependencies.**

Local Agent provides proven patterns for coordinating multiple AI agents using
any locally hosted model that implements the OpenAI chat completions API.
Everything runs on your machine — intent extraction, semantic caching, agent
routing, and dispatch. No data ever leaves your network.

Built for privacy-focused use cases where your data stays yours.

Written in Rust for performance and zero-cost abstractions.

Proven patterns included: capability graphs, semantic intent extraction, agent routing, and caching.

## Quick Start

```bash
# Extract intent (uses defaults — no flags needed)
cargo run -- extract "Find the Q1 sales report"

# Override the LLM model
cargo run -- extract "Find the Q1 sales report" --model-llm qwen3.5:0.8b

# Enable semantic cache (uses defaults for both providers)
cargo run -- extract "Find the Q1 sales report" --cache

# Full customization
cargo run -- extract "Find the Q1 sales report" \
  --model-llm qwen3.5:2b \
  --model-embedding nomic-embed-text \
  --cache \
  --cache-store sqlite

# Run tests
cargo test -v                              # All tests (unit + integration)
cargo test -- --include-ignored            # Include ignored integration tests
```

## CLI Options

All options are optional. Sensible defaults are applied:

| Option | Default | Description |
|---|---|---|
| `--model-llm` | `qwen3.5:2b` | Model name for the LLM provider |
| `--model-embedding` | `nomic-embed-text` | Model name for the embedding provider |
| `--host` | `localhost` | Override host for all providers |
| `--cache` | `false` | Enable semantic intent cache |
| `--cache-store` | `memory` | Cache persistence backend (`memory` or `sqlite`) |
| `--cache-path` | `~/.local/share/coordination-patterns/cache.db` | Path to SQLite cache database |

The embedding model is only used when `--cache` is enabled.
When `--cache-store sqlite` is used, cached entries persist across restarts.

## Patterns

### 1. Capability Graph Router
Route requests to specialized agents using `(action, resource)` → `agent` lookup.

```rust
use coordination_patterns::capability_router::{AgentRouter, RoutingIntent};

let mut router = AgentRouter::new();
let intent = RoutingIntent {
    action: "find".into(),
    resource: "sales_report".into(),
    parameters: serde_json::json!({"quarter": "Q1"}),
};
let result = router.route_request(&intent);
// → "Routing to SalesAgent with params: {"quarter":"Q1"}"
```

### 2. LLM Interface (Swappable Backends)
Talk to any OpenAI-compatible LLM without hardcoding endpoints.

```rust
use coordination_patterns::llm_interface::{LLMClient, LLMConfig};

let client = LLMClient::new(LLMConfig::default());
let response = client.chat(&[
    serde_json::json!({"role": "user", "content": "Hello"}),
]).unwrap();
```

### 3. Semantic Intent Extraction (Full Pipeline)
Natural language → LLM extracts structured intent → route to agent.

```rust
use coordination_patterns::intent_extractor::IntentExtractor;
use coordination_patterns::llm_interface::LLMConfig;

let extractor = IntentExtractor::new(LLMConfig::default());
let result = extractor.process("Find the Q1 sales report").unwrap();
// Internally:
// 1. LLM extracts: {action: "find", resource: "sales_report", parameters: {"quarter": "Q1"}}
// 2. Router dispatches to SalesAgent
// 3. Returns result
```

### 4. Semantic Intent Cache
Bypass the LLM for previously seen (or similar) queries. Requires a separate embedding model.

```rust
use coordination_patterns::intent_extractor::IntentExtractor;
use coordination_patterns::llm_interface::{LLMConfig, EmbeddingConfig};
use coordination_patterns::semantic_cache::CacheStore;

let extractor = IntentExtractor::builder()
    .llm_config(LLMConfig::default())
    .embed_config(EmbeddingConfig::default())
    .cache_enabled(true)
    .cache_store(CacheStore::Memory)
    .build();

// First call — hits LLM (~2s)
extractor.process("Find the Q1 sales report").unwrap();
// Second call — cache hit (~0.01s)
extractor.process("Find the Q1 sales report").unwrap();
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
│   (in-memory/sqlite)│   │ (Pattern #2) │     qwen3.5:0.8b
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
  --model-llm qwen3.5:2b           ──► LLMClient  ──► qwen3.5:2b
  --model-embedding nomic-embed-text ─► EmbeddingClient ──► nomic-embed-text
```

## Project Structure

```text
local-agent/
  Cargo.toml
  src/
    lib.rs                       # library exports
    main.rs                      # CLI (clap-based)
    capability_router.rs         # Pattern #1
    llm_interface.rs             # Pattern #2 (LLMConfig, EmbeddingConfig, clients)
    intent_extractor.rs          # Pattern #3
    semantic_cache.rs            # Pattern #4 (SemanticCache, cosine_similarity, stores)
  tests/
    test_capability_router.rs    # Unit tests
    test_llm_interface.rs        # Unit tests
    test_intent_extractor.rs     # Unit tests
    test_semantic_cache.rs       # Unit tests
    test_semantic_cache_utils.rs # Unit tests
    integration_tests.rs         # Integration tests (SQLite + Ollama)
```

Built in Rust with [`serde`](https://serde.rs/), [`clap`](https://docs.rs/clap/), [`ureq`](https://docs.rs/ureq/), [`rusqlite`](https://docs.rs/rusqlite/), and [`schemars`](https://docs.rs/schemars/).

## Quality Standards

- **Type safety**: Full static typing with Rust's type system — no runtime type errors.
- **Unit tests**: Every feature has unit tests in `tests/`.
- **Integration tests**: Features involving external subsystems (LLM API calls, SQLite I/O) have integration tests.
- **Zero-cost abstractions**: No unnecessary allocations, sync-first design, minimal dependencies.

## Adding New Patterns

1. Create a new module under `src/<name>.rs`
2. Add tests under `tests/`
3. Export from `src/lib.rs`
