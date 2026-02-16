#!/usr/bin/env bash
set -euo pipefail

echo "=== cargo check ==="
cargo check

echo "=== cargo test ==="
cargo test --all-features

echo "=== cargo fmt ==="
cargo fmt --check

echo "=== cargo clippy ==="
cargo clippy -- -D warnings

echo "=== LOC check ==="
bash "$(dirname "$0")/loc-check.sh"

echo "=== All validation gates passed ==="
