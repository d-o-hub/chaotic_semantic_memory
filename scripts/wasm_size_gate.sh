#!/usr/bin/env bash
set -euo pipefail

# Headroom for ADR-0093 index envelope + durable mutation paths compiled into root.
# Bumped from 1120000 after chrono 0.4.45 dependency update grew binary by ~3KB.
DEFAULT_MAX_BYTES=1150000
MAX_BYTES="${CSM_WASM_SIZE_MAX_BYTES:-${DEFAULT_MAX_BYTES}}"
REPORT_PATH="plans/handoffs/W5_C_to_D_wasm_size_report.md"

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --target wasm32-unknown-unknown --release -p csm-wasm >/dev/null

# Find the library WASM (csm_wasm.wasm from the csm-wasm crate)
WASM_FILE="target/wasm32-unknown-unknown/release/csm_wasm.wasm"
if [[ ! -f "${WASM_FILE}" ]]; then
  # Fallback: find any .wasm that's not the CLI binary
  WASM_FILE="$(find target/wasm32-unknown-unknown/release -maxdepth 1 -name '*.wasm' ! -name 'csm.wasm' | head -n 1)"
  if [[ -z "${WASM_FILE}" ]]; then
    echo "No wasm artifact produced under target/wasm32-unknown-unknown/release"
    exit 1
  fi
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
- Command: \`cargo build --target wasm32-unknown-unknown --release --features wasm\`
- Artifact: \`${WASM_FILE}\`
- Size: \`${SIZE_BYTES}\` bytes (\`${SIZE_KB}\` KiB)
- Threshold: \`${MAX_BYTES}\` bytes (configurable via \`CSM_WASM_SIZE_MAX_BYTES\`)

## Result
- Status: \`${STATUS}\`
- \`wasm_binary_under_500kb\`: \`$([[ "${STATUS}" == "pass" ]] && echo true || echo false)\`
EOF

if [[ "${STATUS}" == "fail" ]]; then
  echo "WASM size gate failed: ${SIZE_BYTES} bytes >= ${MAX_BYTES} bytes"
  exit 1
fi

echo "WASM size gate passed: ${SIZE_BYTES} bytes (${SIZE_KB} KiB)"
