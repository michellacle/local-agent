#!/usr/bin/env bash
# Generates docs/domain-language.md and docs/domain-model.mmd from rustdoc comments.
# Usage: ./scripts/generate-domain-language.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"
cargo run --bin generate-domain-language --quiet
