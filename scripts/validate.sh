#!/usr/bin/env bash
set -euo pipefail

MAX_SRC_LOC=500
WASM_TARGET="wasm32-unknown-unknown"

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test --all-targets"
cargo test --all-targets

echo "==> Source file LOC gate (< ${MAX_SRC_LOC})"
for file in src/*.rs; do
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

echo "Validation complete."
