#!/usr/bin/env bash
set -euo pipefail
WASM_DIR="./wasm"
CHECKED_IN_DTS="${WASM_DIR}/chaotic_semantic_memory.d.ts"
TEMP_PKG_DIR=$(mktemp -d)
"${WASM_DIR}/../scripts/build-wasm.sh" dev-web "${TEMP_PKG_DIR}" > /dev/null 2>&1
GENERATED_DTS="${TEMP_PKG_DIR}/chaotic_semantic_memory.d.ts"
# Omit only compiler-generated closure-invocation declarations, then sort to
# handle non-deterministic ordering. Those helpers are named after the
# closure's Rust signature and carry a crate-hash prefix whose value depends on
# the wasm-bindgen/rustc pair that produced them (e.g.
# `wasm_bindgen__convert__closures_____invoke__hbf57a2c43ac1d931` before,
# `wasm_bindgen_847cf8bb5373c819___convert__closures…` after a toolchain bump),
# so they churn without any public API change. Everything else in the
# declaration file — public exports, `InitOutput` entries, `__wbindgen_*`
# ABI functions — must still match the checked-in snapshot.
filter_dts() { grep -vE '^[[:space:]]*readonly wasm_bindgen(_[[:xdigit:]]+)?__+convert__+closures__+invoke__' | sort; }
if diff -u <(filter_dts < "${CHECKED_IN_DTS}") <(filter_dts < "${GENERATED_DTS}"); then
    echo "OK"
    rm -rf "${TEMP_PKG_DIR}"
else
    echo "STALE"
    rm -rf "${TEMP_PKG_DIR}"
    exit 1
fi
