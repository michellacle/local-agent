#!/usr/bin/env bash
# Combined coverage script — runs unit + integration tests and produces a single report.
# Usage: ./scripts/check-coverage.sh [--unit-only]
#
# Without --unit-only, requires Ollama to be running for integration tests.
# Requires: cargo-llvm-cov

set -e

IGNORE_REGEX='tests/|/src/bin/|semantic_cache_sqlite'

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
  --lib \
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
  --lib \
  --test integration_tests \
  --test test_semantic_cache_sqlite \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --fail-under-lines 50 \
  --lcov \
  --output-path docs/coverage.lcov

cargo llvm-cov \
  --lib \
  --test integration_tests \
  --test test_semantic_cache_sqlite \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --json \
  --output-path docs/coverage.json

cargo llvm-cov \
  --lib \
  --test integration_tests \
  --test test_semantic_cache_sqlite \
  --ignore-filename-regex="$IGNORE_REGEX" \
  --html \
  --output-dir docs/coverage-html

echo "Combined coverage reports written to docs/coverage.lcov, docs/coverage.json, docs/coverage-html/"
echo ""
echo "All coverage checks passed."
