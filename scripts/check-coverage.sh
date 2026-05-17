#!/usr/bin/env bash
# Combined coverage script — runs unit + integration tests and produces a single report.
# Usage: ./scripts/check-coverage.sh [--unit-only]
#
# Without --unit-only, requires Ollama to be running for integration tests.
# Requires: cargo-llvm-cov

set -e

UNIT_TESTS=(
  --test test_capability_router \
  --test test_intent_extractor \
  --test test_llm_interface \
  --test test_semantic_cache \
  --test test_semantic_cache_utils \
  --test test_semantic_cache_mock
)

INTEGRATION_TESTS=(
  --test integration_tests \
  --test test_semantic_cache_sqlite
)

IGNORE_REGEX='tests-unit|tests-integration|/src/bin/|semantic_cache_sqlite'

UNIT_ONLY=false
if [ "$1" = "--unit-only" ]; then
  UNIT_ONLY=true
fi

if ! command -v cargo-llvm-cov &>/dev/null; then
  echo "FAIL: cargo-llvm-cov not installed (cargo install cargo-llvm-cov)"
  exit 1
fi

# --- Unit coverage (fast gate) ---
echo "=== Unit coverage (min 55% lines) ==="
cargo llvm-cov \
  "${UNIT_TESTS[@]}" \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --fail-under-lines 55 \
  --summary-only
echo "Unit coverage OK."

if [ "$UNIT_ONLY" = true ]; then
  echo "Done (--unit-only)."
  exit 0
fi

# --- Combined coverage (unit + integration) ---
echo ""
echo "=== Combined coverage (unit + integration, min 50% lines) ==="
cargo llvm-cov \
  "${UNIT_TESTS[@]}" \
  "${INTEGRATION_TESTS[@]}" \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --fail-under-lines 50 \
  --lcov \
  --output-path docs/coverage.lcov

cargo llvm-cov \
  "${UNIT_TESTS[@]}" \
  "${INTEGRATION_TESTS[@]}" \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --json \
  --output-path docs/coverage.json

cargo llvm-cov \
  "${UNIT_TESTS[@]}" \
  "${INTEGRATION_TESTS[@]}" \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --html \
  --output-dir docs/coverage-html

echo "Combined coverage reports written to docs/coverage.lcov, docs/coverage.json, docs/coverage-html/"
echo ""
echo "All coverage checks passed."
