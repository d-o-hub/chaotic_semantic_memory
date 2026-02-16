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
