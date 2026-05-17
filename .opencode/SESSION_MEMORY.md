# Session Memory

## Goal
- Refactor `IntentExtractor` and `SemanticCache` for improved testability and separation of concerns.
- Implement Supervisor Architecture for multi-agent task delegation.

## Constraints & Preferences
- No third-party dependencies for new binaries/scripts.
- Commits should be made locally but not pushed until the refactor series is complete.
- Unit test coverage must remain above 55%, excluding `tests-unit/`, `tests-integration/`, `src/bin/`, and `semantic_cache_sqlite`.
- SQLite is treated as a third-party dependency; tests go in integration suite.

## Progress
### Done
- Added `///` rustdoc descriptions to all domain entities and fields.
- Created `src/bin/generate-domain-language.rs` to auto-generate `docs/domain-language.md` and `docs/domain-model.mmd`.
- Updated coverage scripts to exclude `src/bin/` using `--ignore-filename-regex='tests-unit|/src/bin/'`.
- Added HTML coverage report generation (`docs/coverage-html/`) and updated `.gitignore`.
- Updated `README.md` with `--bin local-agent` usage and semantic cache instructions.
- Renamed `config` to `llm_config` in `IntentExtractor::new` (commit `6ce1d87`).
- Refactored `IntentExtractor::new` to accept `LLMClient` and `EmbeddingClient` instead of configs (shifts client init to callers).
- Refactored `IntentExtractor::new` to accept `Option<Box<dyn SemanticCache>>` instead of `cache_store`/`cache_store_path`/`cache_enabled`.
- Split `SemanticCache` into a trait with three implementations: `InMemorySemanticCache`, `SqliteSemanticCache`, `MockSemanticCache`.
- Moved `SqliteSemanticCache` to separate `src/semantic_cache_sqlite.rs` module to keep unit coverage above 55%.
- Moved sqlite tests from `integration_tests.rs` to `tests-integration/test_semantic_cache_sqlite.rs`.
- Added 10 mock cache unit tests in `tests-unit/test_semantic_cache_mock.rs`.
- Created `scripts/check-coverage.sh` for combined unit + integration coverage reports.
- Updated pre-commit hook and `check-quality.sh` with new test targets and 55% unit coverage threshold.
- Introduced `LLMClientTrait` and `EmbeddingClientTrait` in `src/llm_interface.rs` for testability.
- Refactored `IntentExtractor` to accept `Box<dyn LLMClientTrait>` and `Option<Box<dyn EmbeddingClientTrait>>`.
- Updated `main.rs` and integration tests to use trait-based constructor.
- Added 10 `IntentExtractor::extract` unit tests with `MockLLMClient`, `MockEmbeddingClient`, and `InMemorySemanticCache`.
- Raised unit coverage threshold from 43% to 55% (actual: 55.29%).
- Implemented `WorkerAgent` trait, `Supervisor` struct with builder pattern, `WorkflowStep`, `WorkflowStatus`, `WorkflowResult`, `AuditEntry`.
- Added 22 unit tests for supervisor covering: lifecycle, sequential delegation, conditional branching, fault handling, structured communication, audit trail, separation of concerns.
- Built `LoanOrchestratorAgent` with mock sub-agents (`MockDocumentValidator`, `MockCreditChecker`, `MockRiskAssessor`) mirroring the book's example.
- Added 5 integration tests for loan workflow: full approval, rejected invalid docs, rejected low credit, audit trail verification, sequential delegation.
- Fixed `WorkflowResult::final_output` to derive from audit trail (last successful step) instead of HashMap iteration.

### In Progress
- (none)

### Blocked
- (none)

## Key Decisions
- Use pure Rust (std only) for the domain language generator to maintain zero third-party dependencies.
- Use absolute path regex `/src/bin/` for `cargo-llvm-cov` to correctly exclude the generator binary from coverage metrics.
- Keep `docs/coverage-html/` out of version control.
- Shift client and cache initialization responsibility from `IntentExtractor` to callers for better testability.
- Split `SqliteSemanticCache` into its own module and exclude it from unit coverage since it depends on rusqlite (third-party).
- Treat SQLite as a third-party dependency like Ollama; tests belong in integration suite.
- Add `MockSemanticCache` for fast, dependency-free unit testing of cache-dependent code.
- Introduce `LLMClientTrait` and `EmbeddingClientTrait` to enable mocking of LLM/embedding clients in unit tests.
- Supervisor uses builder pattern with validation (no empty teams, no duplicate workers, no unregistered worker references).
- `WorkflowStep` conditions are `Box<dyn Fn(&str) -> bool>` — not serializable, evaluated on previous step's output.
- `WorkflowResult::final_output` is derived from the last successful audit entry, not HashMap iteration (non-deterministic order).
- `LoanOrchestratorAgent` holds concrete mock types and builds a fresh `Supervisor` per call to avoid trait object cloning.

## Next Steps
- Phase 4: State checkpointing + resume (TC14-TC16)
- Phase 6: LLM-driven dynamic planning (TC24-TC26)
- Wire `LoanOrchestratorAgent` into `AgentRouter` for end-to-end routing from user request.

## Critical Context
- Branch is at `e3cb630` (session memory commit), pushed to `origin/main`.
- `cargo-llvm-cov` requires `--ignore-filename-regex='tests-unit|tests-integration|/src/bin/|semantic_cache_sqlite'` in coverage scripts.
- Pre-commit hook runs unit coverage at 55% threshold; `scripts/check-coverage.sh` runs combined at 50%.
- HTML coverage report is generated at `docs/coverage-html/html/index.html`.
- `SemanticCache` trait methods: `lookup`, `store`, `size`, `clear`, `close`, `threshold`, `entries`.
- `IntentExtractor::new` signature: `(Box<dyn LLMClientTrait>, Option<Box<dyn EmbeddingClientTrait>>, Option<Box<dyn SemanticCache>>) -> Self`.
- `LLMClientTrait` methods: `chat`, `structured_chat`.
- `EmbeddingClientTrait` methods: `embed`.
- `WorkerAgent` trait methods: `name() -> &str`, `execute(input: &str) -> Result<String, String>`.
- `Supervisor::builder(goal)` -> `SupervisorBuilder` -> `add_worker()` -> `add_step()` -> `build()` -> `Supervisor` -> `execute()` -> `WorkflowResult`.
- `WorkflowStep::new(name, worker_name)` -> `.with_condition(fn)` -> `.non_critical()` -> builder.
- `LoanOrchestratorAgent::process(application_data)` -> `WorkflowResult` with 3 steps: document_validation, credit_check, risk_assessment.
- `LoanOrchestratorAgent::decision_string(&WorkflowResult)` -> human-readable decision.

## Relevant Files
- `src/intent_extractor.rs`: Refactored to accept trait objects for clients and cache.
- `src/llm_interface.rs`: Contains `LLMClientTrait`, `EmbeddingClientTrait`, and concrete implementations.
- `src/semantic_cache.rs`: Contains `SemanticCache` trait, `InMemorySemanticCache`, `MockSemanticCache`, `CachedEntry`, `cosine_similarity`, `find_best_match`.
- `src/semantic_cache_sqlite.rs`: Contains `SqliteSemanticCache` (excluded from unit coverage).
- `src/supervisor.rs`: Contains `WorkerAgent` trait, `Supervisor`, `SupervisorBuilder`, `WorkflowStep`, `WorkflowStatus`, `WorkflowResult`, `AuditEntry`.
- `src/loan_orchestrator.rs`: Contains `LoanOrchestratorAgent`, `MockDocumentValidator`, `MockCreditChecker`, `MockRiskAssessor`.
- `src/lib.rs`: Exports new types and modules.
- `src/main.rs`: Updated callers to construct trait objects before passing to `IntentExtractor`.
- `tests-unit/test_supervisor.rs`: 22 tests covering all Phase 1-3 supervisor functionality.
- `tests-unit/test_intent_extractor.rs`: 12 tests covering `extract` method with mock clients.
- `tests-unit/test_semantic_cache.rs`: Updated to use `InMemorySemanticCache`.
- `tests-unit/test_semantic_cache_mock.rs`: 10 mock cache tests.
- `tests-integration/test_loan_orchestrator.rs`: 5 loan workflow integration tests.
- `tests-integration/test_semantic_cache_sqlite.rs`: 9 sqlite-specific tests.
- `tests-integration/integration_tests.rs`: Updated `make_extractor`/`make_cached_extractor`, removed duplicate sqlite tests.
- `docs/supervisor-architecture-plan.md`: Full implementation plan with test cases and phases.
- `scripts/check-coverage.sh`: Script for combined unit + integration coverage reports.
- `scripts/check-quality.sh` & `.git/hooks/pre-commit`: Updated with new test targets and 55% coverage threshold.
- `Cargo.toml`: Added `test_supervisor`, `test_loan_orchestrator` test targets.
