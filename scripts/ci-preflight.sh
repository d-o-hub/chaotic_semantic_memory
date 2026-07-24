#!/usr/bin/env bash
# scripts/ci-preflight.sh — Fast "will CI pass?" check (< 60s)
# Runs only the checks that block merge. Use validate.sh for the full suite.
# Usage: ./scripts/ci-preflight.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
pass() { echo " ok: $1"; }

echo "==> fmt"
if cargo fmt --all -- --check; then
  pass "fmt"
else
  fail "fmt (run: cargo fmt --all)"
fi

echo "==> clippy"
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -5
if [[ ${PIPESTATUS[0]} -eq 0 ]]; then
  pass "clippy"
else
  fail "clippy"
fi

echo "==> test compile"
cargo test --no-run --all-features --quiet 2>&1 | grep -i "error" || pass "test compile"

echo "==> LOC gate"
OVER=$(find src crates -name '*.rs' -not -path '*/target/*' -exec sh -c 'wc -l < "$1"' _ {} \; -print | paste - - | awk '$1 > 500 {print $2 ": " $1 " lines"}')
if [[ -n "$OVER" ]]; then
  fail "LOC gate"
  echo "$OVER"
else
  pass "LOC gate"
fi

echo "==> wasm check"
if rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
  if cargo check --target wasm32-unknown-unknown --features wasm --quiet; then
    pass "wasm"
  else
    fail "wasm check"
  fi
else
  echo "skip: wasm target not installed"
fi

echo ""
if [[ $FAILED -ne 0 ]]; then
  echo "CI PREFLIGHT FAILED — fix before pushing"
  exit 1
fi
echo "CI PREFLIGHT PASSED — safe to push"
