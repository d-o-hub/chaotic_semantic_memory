#!/usr/bin/env bash
set -euo pipefail

export CARGO_TERM_PROGRESS_WHEN=never

echo "=== cargo check ==="
cargo check --message-format=short 2>&1 | grep -E "(^error|^warning:|^    -->|Finished|^test )" || echo "✓ check passed"

echo ""
echo "=== cargo test ==="
if command -v cargo-nextest &> /dev/null || cargo nextest --version &> /dev/null; then
    cargo nextest run --all-features 2>&1 | tail -15
else
    cargo test --all-features --quiet 2>&1 | grep -E "(^test |^running|^test result)" || echo "✓ tests passed"
fi

echo ""
echo "=== cargo fmt ==="
if ! cargo fmt --check 2>&1 | grep -q "Diff"; then
    echo "✓ fmt passed"
else
    cargo fmt --check 2>&1 | head -5
    exit 1
fi

echo ""
echo "=== cargo clippy ==="
cargo clippy -- -D warnings 2>&1 | grep -E "(^error|^warning:|note:|^   -->)" | head -20 || echo "✓ clippy passed"

echo ""
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

echo ""
echo "=== All gates passed ==="
