#!/usr/bin/env bash
# Pre-commit hook: Fast checks only (fmt + LOC gate + CHANGELOG)
# For full validation, run: scripts/validate.sh

set -euo pipefail

MAX_SRC_LOC=500

echo "Running pre-commit checks..."

# Check formatting
echo " → Checking formatting..."
cargo fmt -- --check

# LOC gate (fast)
echo " → Checking LOC limits (< ${MAX_SRC_LOC})..."
for file in $(find src -name '*.rs'); do
  loc="$(wc -l < "${file}")"
  if [ "${loc}" -gt "${MAX_SRC_LOC}" ]; then
    echo "❌ LOC gate failed: ${file} has ${loc} lines (max: ${MAX_SRC_LOC})"
    exit 1
  fi
done

# CHANGELOG duplicate header guardrail (prevents release workflow failures)
echo " → Checking CHANGELOG format..."
if [ -f "CHANGELOG.md" ]; then
  # Check for duplicate version headers (each version should appear exactly once)
  DUPLICATES=$(grep "^## \\[" CHANGELOG.md | cut -d'[' -f2 | cut -d']' -f1 | sort | uniq -d)
  if [ -n "$DUPLICATES" ]; then
    echo "❌ Duplicate CHANGELOG headers found: $DUPLICATES"
    echo "   Each version should have exactly one '## [VERSION] - YYYY-MM-DD' header"
    exit 1
  fi
  
  # Check that version headers have dates
  NO_DATE=$(grep "^## \\[" CHANGELOG.md | grep -v " - [0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\}" | grep -v "\\[Unreleased\\]" || true)
  if [ -n "$NO_DATE" ]; then
    echo "❌ CHANGELOG headers missing dates:"
    echo "$NO_DATE"
    echo "   Format: ## [VERSION] - YYYY-MM-DD"
    exit 1
  fi
fi

# Docs sync check
echo " → Checking docs sync..."
bash scripts/sync-docs.sh --check

# Clippy lint (catches CI failures early)
echo " → Checking clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "✅ Pre-commit checks passed!"
