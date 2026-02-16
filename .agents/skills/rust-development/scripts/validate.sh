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

echo "=== LOC check (max 500) ==="
fail=0
for file in src/*.rs; do
  loc=$(wc -l < "$file")
  if [ "$loc" -gt 500 ]; then
    echo "FAIL $file: $loc LOC"
    fail=1
  else
    echo "OK   $file: $loc LOC"
  fi
done
if [ "$fail" -ne 0 ]; then
  echo "LOC check failed"
  exit 1
fi

echo "=== All gates passed ==="
