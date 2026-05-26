#!/usr/bin/env bash
set -euo pipefail
WASM_DIR="./wasm"
CHECKED_IN_DTS="${WASM_DIR}/chaotic_semantic_memory.d.ts"
TEMP_PKG_DIR=$(mktemp -d)
wasm-pack build --dev --target web --out-dir "${TEMP_PKG_DIR}" -- --features wasm > /dev/null 2>&1
GENERATED_DTS="${TEMP_PKG_DIR}/chaotic_semantic_memory.d.ts"
if diff -u "${CHECKED_IN_DTS}" "${GENERATED_DTS}"; then
    echo "OK"
    rm -rf "${TEMP_PKG_DIR}"
else
    echo "STALE"
    rm -rf "${TEMP_PKG_DIR}"
    exit 1
fi
