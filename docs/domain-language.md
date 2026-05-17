# Domain Language

Auto-generated vocabulary of domain entities from source code.

## ActionType

**Kind:** Enum


## AgentRouter

**Kind:** Struct

Maps (action, resource) pairs to agent names and dispatches requests.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `capability_graph` | `std::collections::HashMap<(String, String), String>` | Internal lookup table from (action, resource) to agent name. |

## CacheStore

**Kind:** Trait

Persistent or in-memory backend for storing cached entries.


## CachedEntry

**Kind:** Struct

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `query` | `String` | The original natural language query text. |
| `embedding` | `Vec<f64>` | The embedding vector of the query. |
| `intent` | `RoutingIntent` | The resolved routing intent for this query. |
| `created_at` | `f64` | Unix timestamp when the entry was created. |
| `hit_count` | `u64` | Number of times this entry has been matched on lookup. |

## EmbeddingClient

**Kind:** Struct

HTTP client for generating text embeddings from an embedding provider.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `config` | `EmbeddingConfig` | Embedding provider configuration. |
| `agent` | `ureq::Agent` | Underlying HTTP agent. |

## EmbeddingConfig

**Kind:** Struct

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `provider` | `String` | Identifier for the embedding provider. |
| `base_url` | `String` | Base URL of the embedding API endpoint. |
| `model` | `String` | Embedding model name. |
| `timeout` | `u64` | Request timeout in seconds. |

## IntentExtractor

**Kind:** Struct

Orchestrates the full pipeline: extracts intent from natural language, checks the semantic cache, and routes to the correct agent.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `client` | `LLMClient` | LLM client used for intent extraction. |
| `embed_client` | `Option<EmbeddingClient>` | Embedding client for generating query vectors (only when cache is enabled). |
| `router` | `AgentRouter` | Router that maps resolved intents to agent names. |
| `cache` | `Option<SemanticCache>` | Optional semantic cache for bypassing the LLM on similar queries. |
| `cache_enabled` | `bool` | Whether semantic caching is active. |

## LLMClient

**Kind:** Struct

HTTP client for sending chat and structured-output requests to an LLM.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `config` | `LLMConfig` | Connection and model configuration. |
| `agent` | `ureq::Agent` | Underlying HTTP agent. |

## LLMConfig

**Kind:** Struct

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `provider` | `String` | Identifier for the provider (e.g., "ollama-local", "openai_compat"). |
| `base_url` | `String` | Base URL of the provider's API endpoint. |
| `model` | `String` | Model name to use for completions. |
| `api_key` | `String` | API key for authentication. |
| `temperature` | `f64` | Sampling temperature for response generation. |
| `max_tokens` | `u32` | Maximum number of tokens in the response. |
| `timeout` | `u64` | Request timeout in seconds. |
| `deterministic` | `bool` | When true, forces deterministic output (temperature=0, seed=0). |

## MemoryCacheStore

**Kind:** Struct

In-memory implementation of `CacheStore` using a thread-safe vector.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `entries` | `std::sync::Mutex<Vec<CacheTuple>>` | Thread-safe list of cached tuples. |

## ResourceType

**Kind:** Enum


## RoutingIntent

**Kind:** Struct

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `action` | `ActionType` | The action to perform. |
| `resource` | `ResourceType` | The resource type the action targets. |
| `parameters` | `Value` | Arbitrary key-value parameters extracted from the request. |

## SemanticCache

**Kind:** Struct

Semantic cache that matches incoming query embeddings against stored entries using cosine similarity.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `threshold` | `f64` | Minimum cosine similarity score required for a cache hit. |
| `max_size` | `usize` | Maximum number of entries before eviction is triggered. |
| `backend` | `Box<dyn CacheStore>` | Persistent or in-memory storage backend. |
| `entries` | `Vec<CachedEntry>` | In-memory index of all cached entries for fast lookup. |

## SqliteCacheStore

**Kind:** Struct

SQLite-backed implementation of `CacheStore` for persistent caching.

### Fields

| Field | Type | Description |
| ----- | ---- | ----------- |
| `conn` | `std::sync::Mutex<rusqlite::Connection>` | Thread-safe SQLite database connection. |

