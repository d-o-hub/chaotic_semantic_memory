#!/usr/bin/env bash
# Fixture for scripts/crates-io-index-path.sh (issue #974). The expected paths
# are the ones measured on the live sparse index on 2026-10-31.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/crates-io-index-path.sh"
PASS=0; FAIL=0
check() {
  local label="$1" expected="$2" actual="$3"
  if [[ "${actual}" == "${expected}" ]]; then PASS=$((PASS+1)); echo "✅ ${label}";
  else FAIL=$((FAIL+1)); echo "❌ ${label}: expected ${expected}, got ${actual}" >&2; fi
}
check "1-char" "1/a" "$(bash "${SUBJECT}" a)"
check "2-char" "2/io" "$(bash "${SUBJECT}" io)"
check "3-char" "3/s/syn" "$(bash "${SUBJECT}" syn)"
check "4-char" "ra/nd/rand" "$(bash "${SUBJECT}" rand)"
check "duckdb" "cs/m-/csm-duckdb" "$(bash "${SUBJECT}" csm-duckdb)"
check "root" "ch/ao/chaotic_semantic_memory" "$(bash "${SUBJECT}" chaotic_semantic_memory)"
rc=0; bash "${SUBJECT}" >/dev/null 2>&1 || rc=$?
check "no argument is exit 2" "2" "${rc}"
echo "crates-io-index-path fixture: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
