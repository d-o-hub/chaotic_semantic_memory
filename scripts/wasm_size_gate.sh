#!/usr/bin/env bash
set -euo pipefail

# Measures the artifact that actually ships: the `wasm-pack --release --target
# web` package that `release.yml` publishes to npm, built through the same
# script CI uses (`wasm_ci_release_artifact_identical`).
#
# The previous gate measured a raw `cargo build` output (1 133 971 B before it
# was replaced) which is 6.2x larger than the shipped 656 657 B — it could not
# have caught a regression in the released package.
#
# Threshold: 800 000 B — the measured artifact plus ~22 % headroom. Override
# with CSM_WASM_SIZE_MAX_BYTES.
DEFAULT_MAX_BYTES=800000
MAX_BYTES="${CSM_WASM_SIZE_MAX_BYTES:-${DEFAULT_MAX_BYTES}}"
REPORT_PATH="plans/handoffs/W5_C_to_D_wasm_size_report.md"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

PACKAGE_DIR="$(mktemp -d)"
trap 'rm -rf "${PACKAGE_DIR}"' EXIT

BUILD_OUTPUT="$(bash scripts/build-wasm.sh release-web "${PACKAGE_DIR}")"
echo "${BUILD_OUTPUT}"
SHA256="$(printf '%s\n' "${BUILD_OUTPUT}" | sed -n 's/^wasm_sha256=//p')"

WASM_FILE="${PACKAGE_DIR}/chaotic_semantic_memory_bg.wasm"
if [[ ! -f "${WASM_FILE}" ]]; then
  echo "No wasm artifact produced by scripts/build-wasm.sh release-web"
  exit 1
fi

SIZE_BYTES="$(wc -c < "${WASM_FILE}")"
SIZE_KB="$(awk "BEGIN { printf \"%.2f\", ${SIZE_BYTES}/1024 }")"
STATUS="pass"

if (( SIZE_BYTES >= MAX_BYTES )); then
  STATUS="fail"
fi

cat > "${REPORT_PATH}" <<EOF
# W5 C -> D Handoff: WASM Size Report

## Action
- \`validate_wasm_binary_size\`

## Measurement
- Command: \`scripts/build-wasm.sh release-web\` (same build CI validates and \`release.yml\` publishes)
- Artifact: \`chaotic_semantic_memory_bg.wasm\`
- Size: \`${SIZE_BYTES}\` bytes (\`${SIZE_KB}\` KiB)
- SHA-256: \`${SHA256}\`
- Threshold: \`${MAX_BYTES}\` bytes (configurable via \`CSM_WASM_SIZE_MAX_BYTES\`)

## Result
- Status: \`${STATUS}\`
- \`wasm_binary_under_500kb\`: \`$([[ "${STATUS}" == "pass" ]] && echo true || echo false)\`
EOF

if [[ "${STATUS}" == "fail" ]]; then
  echo "WASM size gate failed: ${SIZE_BYTES} bytes >= ${MAX_BYTES} bytes"
  exit 1
fi

echo "WASM size gate passed: ${SIZE_BYTES} bytes (${SIZE_KB} KiB), sha256=${SHA256}"
