#!/usr/bin/env bash
set -euo pipefail

# Script to ensure that the committed TypeScript definitions match the generated ones.

TMP_DIR="wasm_build_tmp"
COMMITTED_DTS="wasm/chaotic_semantic_memory.d.ts"

echo "==> Regenerating WASM bindings..."
# Note: we use --target web as that is what was used for the current definitions
# and it generates the most comprehensive .d.ts for our needs.
wasm-pack build --target web --out-dir "${TMP_DIR}" -- --features wasm > /dev/null 2>&1

if [[ ! -f "${TMP_DIR}/chaotic_semantic_memory.d.ts" ]]; then
    echo "Error: wasm-pack failed to generate .d.ts file."
    exit 1
fi

# Compare the generated .d.ts with the committed one.
# NOTE: We check if all generated methods exist in the committed file.
# Since we manually refined types, we don't do a full file diff.

MISSING_METHODS=()
GENERATED_DTS="${TMP_DIR}/chaotic_semantic_memory.d.ts"

# Extract method names from generated file using a pattern that matches WASM-bindgen generated TS
# Example: "associate(from: string, to: string, strength: number): Promise<void>;"
# We match word characters followed by an open parenthesis.
METHODS=$(grep -E '^[[:space:]]+\w+\(' "${GENERATED_DTS}" | sed -E 's/^[[:space:]]+(\w+)\(.*/\1/' | sort -u)

for method in $METHODS; do
    # Skip standard wasm-bindgen methods like 'free' and constructor
    if [[ "$method" == "free" || "$method" == "constructor" ]]; then
        continue
    fi
    if ! grep -q "${method}(" "${COMMITTED_DTS}"; then
        MISSING_METHODS+=("${method}")
    fi
done

if [ ${#MISSING_METHODS[@]} -eq 0 ]; then
    echo "✅ WASM TypeScript definitions contain all methods."
    rm -rf "${TMP_DIR}"
else
    echo "❌ WASM TypeScript definitions are MISSING methods!"
    for method in "${MISSING_METHODS[@]}"; do
        echo "  - ${method}"
    done
    echo "Please update ${COMMITTED_DTS}."
    rm -rf "${TMP_DIR}"
    exit 1
fi
