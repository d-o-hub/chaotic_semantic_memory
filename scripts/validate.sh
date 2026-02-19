#!/usr/bin/env bash
set -euo pipefail

MAX_SRC_LOC=500
WASM_TARGET="wasm32-unknown-unknown"

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

# CI applies stricter RUSTFLAGS; this is the minimal local gate
echo "==> cargo test --no-run --all-features (check for warnings)"
if cargo test --no-run --all-features 2>&1 | grep -qi "warning:"; then
  echo "Error: Warnings found in test compilation"
  exit 1
fi

echo "==> cargo test --all-targets"
cargo test --all-targets

echo "==> Source file LOC gate (< ${MAX_SRC_LOC})"
for file in $(find src -name '*.rs'); do
  loc="$(wc -l < "${file}")"
  if [ "${loc}" -gt "${MAX_SRC_LOC}" ]; then
    echo "LOC gate failed: ${file} has ${loc} lines"
    exit 1
  fi
  echo "ok: ${file} (${loc} LOC)"
done

if rustup target list --installed | grep -q "^${WASM_TARGET}\$"; then
  echo "==> cargo check --target ${WASM_TARGET} --features wasm"
  cargo check --target "${WASM_TARGET}" --features wasm
else
  echo "skip: ${WASM_TARGET} target not installed"
fi

if [ -x scripts/wasm_size_gate.sh ]; then
  echo "==> scripts/wasm_size_gate.sh"
  scripts/wasm_size_gate.sh
fi

echo "==> Generating/validating llms-full.txt"
scripts/gen-llms-txt.sh
if ! git diff --quiet llms-full.txt 2>/dev/null; then
  echo "⚠️  llms-full.txt was modified - run 'git add llms-full.txt' to include changes"
fi

echo "Validation complete."
