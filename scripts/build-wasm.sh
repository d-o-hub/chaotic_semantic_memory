#!/usr/bin/env bash
# scripts/build-wasm.sh
#
# Single source of truth for the WASM package build. Every consumer — CI, the
# release workflow, the size gate and the type-freshness check — goes through
# this script so the artifact that ships is the artifact CI validates
# (`wasm_ci_release_artifact_identical`, ADR-0095 Tier 3).
#
# Usage:
#   scripts/build-wasm.sh <dev-nodejs|dev-web|release-web> [OUT_DIR]
#
# Modes:
#   dev-nodejs   unoptimised, `--target nodejs` — fast smoke build for Node
#   dev-web      unoptimised, `--target web`    — type/glue freshness checks
#   release-web  optimised (wasm-opt from Cargo metadata), `--target web`
#                — exactly what `release.yml` publishes to npm
#
# Prints `wasm_package=<dir>`, `wasm_bytes=<n>` and `wasm_sha256=<hex>`.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

MODE="${1:-}"
OUT_DIR="${2:-wasm/pkg}"
OUT_NAME="chaotic_semantic_memory"
CRATE="crates/csm-wasm"

case "${MODE}" in
  dev-nodejs)
    PROFILE_FLAG=(--dev)
    TARGET="nodejs"
    ;;
  dev-web)
    PROFILE_FLAG=(--dev)
    TARGET="web"
    ;;
  release-web)
    PROFILE_FLAG=(--release)
    TARGET="web"
    ;;
  *)
    echo "usage: $0 <dev-nodejs|dev-web|release-web> [OUT_DIR]" >&2
    exit 2
    ;;
esac

command -v wasm-pack >/dev/null 2>&1 || {
  echo "wasm-pack is required (cargo install wasm-pack)" >&2
  exit 1
}

echo "==> wasm-pack build ${CRATE} ${PROFILE_FLAG[*]} --target ${TARGET} --out-dir ${OUT_DIR}"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" wasm-pack build "${CRATE}" \
  "${PROFILE_FLAG[@]}" \
  --target "${TARGET}" \
  --out-dir "${OUT_DIR}" \
  --out-name "${OUT_NAME}"

WASM_FILE="${OUT_DIR}/${OUT_NAME}_bg.wasm"
if [[ ! -f "${WASM_FILE}" ]]; then
  echo "expected artifact missing: ${WASM_FILE}" >&2
  exit 1
fi

BYTES="$(wc -c < "${WASM_FILE}")"
SHA="$(sha256sum "${WASM_FILE}" | cut -d' ' -f1)"
echo "wasm_package=${OUT_DIR}"
echo "wasm_bytes=${BYTES}"
echo "wasm_sha256=${SHA}"
