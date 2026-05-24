#!/usr/bin/env bash
set -euo pipefail

# Script to ensure that the committed TypeScript definitions match the generated ones.

TMP_DIR="wasm_build_tmp"
COMMITTED_DTS="wasm/chaotic_semantic_memory.d.ts"

echo "==> Regenerating WASM bindings..."
# Note: we use --target web as that is what was used for the current definitions
# and it generates the most comprehensive .d.ts for our needs.
wasm-pack build --target web --out-dir "${TMP_DIR}" -- --features wasm

if [[ ! -f "${TMP_DIR}/chaotic_semantic_memory.d.ts" ]]; then
    echo "Error: wasm-pack failed to generate .d.ts file."
    exit 1
fi

# Compare the generated .d.ts with the committed one.
# We ignore whitespace and line endings for robustness.
if diff -uwB "${COMMITTED_DTS}" "${TMP_DIR}/chaotic_semantic_memory.d.ts" > /dev/null; then
    echo "✅ WASM TypeScript definitions are up to date."
    rm -rf "${TMP_DIR}"
    exit 0
else
    echo "❌ WASM TypeScript definitions are OUT OF DATE!"
    echo "Please run wasm-pack build and update ${COMMITTED_DTS}."
    echo "Differences:"
    diff -uwB "${COMMITTED_DTS}" "${TMP_DIR}/chaotic_semantic_memory.d.ts" || true
    rm -rf "${TMP_DIR}"
    exit 1
fi
