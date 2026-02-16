#!/usr/bin/env bash
set -euo pipefail

echo "=== cargo check ==="
cargo check --quiet 2>&1 | tail -20

echo "=== cargo test ==="
cargo test --all-features --quiet 2>&1 | tail -30

echo "=== cargo fmt ==="
cargo fmt --check --quiet 2>&1 | tail -10

echo "=== cargo clippy ==="
cargo clippy --quiet -- -D warnings 2>&1 | tail -20

echo "=== LOC check ==="
bash "$(dirname "$0")/loc-check.sh"

echo "=== All validation gates passed ==="
