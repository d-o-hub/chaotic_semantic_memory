#!/usr/bin/env bash
# Pre-commit hook: Fast checks only (fmt + LOC gate)
# For full validation, run: scripts/validate.sh

set -euo pipefail

MAX_SRC_LOC=500

echo "Running pre-commit checks..."

# Check formatting
echo "  → Checking formatting..."
cargo fmt -- --check

# LOC gate (fast)
echo "  → Checking LOC limits (< ${MAX_SRC_LOC})..."
for file in $(find src -name '*.rs'); do
  loc="$(wc -l < "${file}")"
  if [ "${loc}" -gt "${MAX_SRC_LOC}" ]; then
    echo "❌ LOC gate failed: ${file} has ${loc} lines (max: ${MAX_SRC_LOC})"
    exit 1
  fi
done

# Docs sync check
echo "  → Checking docs sync..."
bash scripts/sync-docs.sh --check

echo "✅ Pre-commit checks passed!"
