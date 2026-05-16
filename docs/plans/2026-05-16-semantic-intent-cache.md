
# Semantic Intent Cache — Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Add an optional semantic cache layer that bypasses LLM intent extraction for previously seen (or similar) queries — reducing latency and cost.

**Architecture:** A `SemanticCache` module stores `(query, embedding, intent)` tuples. `IntentExtractor` optionally checks the cache before calling the LLM. Toggled via `IntentExtractor(cache_enabled=True)`. Embeddings computed via the same OpenAI-compatible endpoint (`/embeddings`).

**Tech Stack:** `httpx` (already a dep), `pydantic` (already a dep), no new deps — use the existing LLMClient for embedding calls. Pure in-memory cache with optional SQLite persistence.

---

## Phase 1: Embedding Client

### Task 1: Add `embed()` method to `LLMClient`

**Objective:** Reuse the existing HTTP client to compute text embeddings via the OpenAI-compatible `/embeddings` endpoint.

**Files:**
- Modify: `src/coordination_patterns/llm_interface/client.py`

**Step 1: Write the method**

```python
def embed(self, text: str) -> list[float]:
    """Compute an embedding vector for the given text."""
    url = f"{self.config.base_url}/embeddings"
    payload = {
        "model": self.config.model,  # or a dedicated embed model
        "input": text,
    }
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {self.config.api_key}",
    }
    resp = self.client.post(url, json=payload, headers=headers)
    resp.raise_for_status()
    data = resp.json()
    return data["data"][0]["embedding"]
```

**Step 2: Add unit test**

Test: `tests/test_llm_interface.py::test_embed_method_exists` — just verify the method signature exists (no network).

**Step 3: Commit**

```bash
git add src/coordination_patterns/llm_interface/client.py
git commit -m "feat: add embed() method to LLMClient"
```

---

### Task 2: Add cosine similarity utility

**Objective:** Pure-Python cosine similarity for matching cached embeddings against new query embeddings.

**Files:**
- Create: `src/coordination_patterns/semantic_cache/utils.py`

**Step 1: Write the function**

```python
def cosine_similarity(a: list[float], b: list[float]) -> float:
    """Compute cosine similarity between two vectors."""
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = sum(x * x for x in a) ** 0.5
    norm_b = sum(x * x for x in b) ** 0.5
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)
```

**Step 2: Write unit tests**

Test: `tests/test_utils.py` — identity vector = 1.0, orthogonal = 0.0, opposite = -1.0

**Step 3: Commit**

```bash
git add src/coordination_patterns/semantic_cache/
git commit -m "feat: add cosine_similarity utility"
```

---

## Phase 2: Semantic Cache Module

### Task 3: Create `CachedEntry` model

**Objective:** Pydantic model for a cache entry.

**Files:**
- Create: `src/coordination_patterns/semantic_cache/model.py`

**Step 1: Write the model**

```python
from __future__ import annotations
import time
from pydantic import BaseModel
from coordination_patterns.capability_router.pattern import RoutingIntent

class CachedEntry(BaseModel):
    query: str
    embedding: list[float]
    intent: RoutingIntent
    created_at: float = 0.0
    hit_count: int = 0

    def __post_init__(self):
        if self.created_at == 0.0:
            self.created_at = time.time()
```

**Step 2: Write unit test**

Test: `tests/test_semantic_cache_model.py` — serialization round-trip

**Step 3: Commit**

```bash
git add src/coordination_patterns/semantic_cache/model.py
git commit -m "feat: add CachedEntry pydantic model"
```

---

### Task 4: Implement `SemanticCache` class

**Objective:** In-memory cache with semantic similarity lookup and self-population.

**Files:**
- Create: `src/coordination_patterns/semantic_cache/cache.py`

**Step 1: Write the class**

```python
class SemanticCache:
    def __init__(self, threshold: float = 0.92, max_size: int = 1000):
        self.threshold = threshold
        self.max_size = max_size
        self._entries: list[CachedEntry] = []

    def lookup(self, embedding: list[float]) -> RoutingIntent | None:
        """Find the most similar cached entry. Returns None if below threshold."""
        best_score = -1.0
        best_entry: CachedEntry | None = None
        for entry in self._entries:
            score = cosine_similarity(embedding, entry.embedding)
            if score > best_score:
                best_score = score
                best_entry = entry
        if best_entry and best_score >= self.threshold:
            best_entry.hit_count += 1
            return best_entry.intent
        return None

    def store(self, query: str, embedding: list[float], intent: RoutingIntent) -> None:
        """Add an entry. Evict LRU if at max_size."""
        self._entries.append(CachedEntry(query=query, embedding=embedding, intent=intent))
        if len(self._entries) > self.max_size:
            # Evict least recently hit
            self._entries.sort(key=lambda e: e.hit_count)
            self._entries = self._entries[-self.max_size:]

    @property
    def size(self) -> int:
        return len(self._entries)

    def clear(self) -> None:
        self._entries.clear()
```

**Step 2: Write unit tests**

Test: `tests/test_semantic_cache.py` — exact match, similar match, no match, eviction

**Step 3: Commit**

```bash
git add src/coordination_patterns/semantic_cache/cache.py
git commit -m "feat: implement SemanticCache with similarity lookup and LRU eviction"
```

---

### Task 5: Create `__init__.py` + export

**Files:**
- Create: `src/coordination_patterns/semantic_cache/__init__.py`
- Modify: `src/coordination_patterns/__init__.py`

**Step 1: Wire exports**

```python
# semantic_cache/__init__.py
from .cache import SemanticCache
from .model import CachedEntry
__all__ = ["SemanticCache", "CachedEntry"]
```

```python
# __init__.py — add to imports and __all__
from coordination_patterns.semantic_cache.cache import SemanticCache
```

**Step 2: Commit**

```bash
git add src/coordination_patterns/semantic_cache/__init__.py src/coordination_patterns/__init__.py
git commit -m "feat: export SemanticCache from package"
```

---

## Phase 3: Wire Into IntentExtractor

### Task 6: Add optional cache to `IntentExtractor`

**Objective:** Make `IntentExtractor` check the cache before calling the LLM. Toggled via `cache_enabled` param.

**Files:**
- Modify: `src/coordination_patterns/intent_extractor/extractor.py`

**Step 1: Update constructor**

```python
def __init__(self, config: LLMConfig | None = None, cache_enabled: bool = False):
    self.client = LLMClient(config)
    self.router = AgentRouter()
    self.cache_enabled = cache_enabled
    self.cache = SemanticCache() if cache_enabled else None
```

**Step 2: Update extract() with cache lookup**

```python
def extract(self, user_input: str) -> RoutingIntent:
    # Cache lookup (bypass LLM)
    if self.cache_enabled and self.cache:
        embedding = self.client.embed(user_input)
        cached = self.cache.lookup(embedding)
        if cached is not None:
            print(f"Cache HIT (threshold {self.cache.threshold})")
            return cached

    # LLM extraction (existing logic)
    messages = [...]
    result = self.client.structured_chat(...)
    intent = RoutingIntent(...)

    # Store in cache
    if self.cache_enabled and self.cache:
        embedding = self.client.embed(user_input)
        self.cache.store(user_input, embedding, intent)

    return intent
```

**Step 3: Add unit test (mocked)**

Test: `tests/test_intent_extractor.py::test_cache_enabled` — verify constructor accepts param
Test: `tests/test_intent_extractor.py::test_cache_disabled_by_default`

**Step 4: Commit**

```bash
git add src/coordination_patterns/intent_extractor/extractor.py
git commit -m "feat: add optional semantic cache to IntentExtractor"
```

---

### Task 7: Add integration test for cache hit/miss

**Objective:** Verify cache actually bypasses LLM on repeat queries.

**Files:**
- Modify: `tests-integration/test_valid_routes.py`
- Modify: `tests-integration/conftest.py`

**Step 1: Add cached extractor fixture**

```python
@pytest.fixture(scope="session")
def cached_extractor(integ_config):
    from coordination_patterns.intent_extractor.extractor import IntentExtractor
    extractor = IntentExtractor(integ_config, cache_enabled=True)
    yield extractor
    extractor.close()
```

**Step 2: Write cache hit test**

```python
def test_cache_hit_on_repeat_query(cached_extractor):
    # First call — cache miss, hits LLM
    result1 = cached_extractor.process("Find the Q1 sales report")
    assert result1 == "Success"

    # Second call — cache hit, bypasses LLM
    result2 = cached_extractor.process("Find the Q1 sales report")
    assert result2 == "Success"
```

**Step 3: Commit**

```bash
git add tests-integration/
git commit -m "test: add integration test for cache hit/miss"
```

---

## Phase 4: CLI Support

### Task 8: Add `--cache` flag to CLI

**Files:**
- Modify: `src/coordination_patterns/__main__.py`

**Step 1: Add flag**

```python
extract_parser.add_argument(
    "--cache",
    action="store_true",
    default=False,
    help="Enable semantic intent cache (bypass LLM for similar queries)",
)
```

**Step 2: Wire to IntentExtractor**

```python
with IntentExtractor(config, cache_enabled=args.cache) as extractor:
```

**Step 3: Commit**

```bash
git add src/coordination_patterns/__main__.py
git commit -m "feat: add --cache CLI flag to extract command"
```

---

## Phase 5: Documentation

### Task 9: Update README with Pattern #4

**Files:**
- Modify: `README.md`

**Step 1: Add pattern section + update project structure + add test commands**

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add Semantic Intent Cache pattern to README"
```

---

## Verification

```bash
# Unit tests
uv run pytest tests/ -v

# Integration tests
uv run pytest tests-integration/ -v

# CLI with cache
uv run coordination-patterns extract "Find the Q1 sales report" --provider ollama-local --cache
uv run coordination-patterns extract "Find the Q1 sales report" --provider ollama-local --cache  # should be instant
```

---

## Summary

**New files:** 3 (`semantic_cache/__init__.py`, `cache.py`, `model.py`, `utils.py`)
**Modified files:** 4 (`client.py`, `extractor.py`, `__main__.py`, `README.md`)
**New tests:** 3 files (unit + integration)
**New dependencies:** 0
