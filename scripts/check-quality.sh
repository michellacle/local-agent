#!/usr/bin/env bash
# Full quality check script.
# Usage: ./scripts/check-quality.sh
# Requires: cargo, clippy, rustfmt, cargo-audit, cargo-llvm-cov
#
# Install optional tools:
#   rustup component add clippy rustfmt
#   cargo install cargo-audit cargo-llvm-cov

set -e

echo "=== 1. Format check ==="
cargo fmt --check
echo "Format OK."

echo "=== 2. Clippy ==="
cargo clippy -- -D warnings
echo "Clippy OK."

echo "=== 3. Unit tests ==="
cargo test \
  --test test_capability_router \
  --test test_intent_extractor \
  --test test_llm_interface \
  --test test_semantic_cache \
  --test test_semantic_cache_utils \
  --test test_semantic_cache_mock \
  --quiet
echo "All unit tests passed."

echo "=== 4. Integration tests (requires Ollama) ==="
cargo test --test integration_tests
echo "All integration tests passed."

echo ""
echo "=== 5. Security audit ==="
if command -v cargo-audit &>/dev/null; then
    cargo audit
    echo "Audit OK."
else
    echo "FAIL: cargo-audit not installed (cargo install cargo-audit)"
    exit 1
fi

echo ""
echo "=== 6. Coverage report (min 35% lines) ==="
if command -v cargo-llvm-cov &>/dev/null; then
    cargo llvm-cov \
      --test test_capability_router \
      --test test_intent_extractor \
      --test test_llm_interface \
      --test test_semantic_cache \
      --test test_semantic_cache_utils \
      --test test_semantic_cache_mock \
      --ignore-filename-regex='tests-unit|/src/bin/|semantic_cache_sqlite' \
      --fail-under-lines 35 \
      --lcov \
      --output-path docs/coverage.lcov
    cargo llvm-cov \
      --test test_capability_router \
      --test test_intent_extractor \
      --test test_llm_interface \
      --test test_semantic_cache \
      --test test_semantic_cache_utils \
      --test test_semantic_cache_mock \
      --ignore-filename-regex='tests-unit|/src/bin/|semantic_cache_sqlite' \
      --json \
      --output-path docs/coverage.json
    cargo llvm-cov \
      --test test_capability_router \
      --test test_intent_extractor \
      --test test_llm_interface \
      --test test_semantic_cache \
      --test test_semantic_cache_utils \
      --test test_semantic_cache_mock \
      --ignore-filename-regex='tests-unit|/src/bin/|semantic_cache_sqlite' \
      --html \
      --output-dir docs/coverage-html
    echo "Coverage reports written to docs/coverage.lcov, docs/coverage.json, docs/coverage-html/"
else
    echo "FAIL: cargo-llvm-cov not installed (cargo install cargo-llvm-cov)"
    exit 1
fi

echo ""
echo "All quality checks passed."
