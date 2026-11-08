#!/usr/bin/env bash
# Fixture for scripts/clean-incomplete-duckdb-unpack.sh (issue #1004).
#
# The failure it repairs is a cached `duckdb-sources/<hash>` tree without the
# `.complete` marker: libduckdb-sys publishes by `fs::rename`, so the next build
# aborts with DirectoryNotEmpty before any test runs. The fixture builds both
# directions — an incomplete tree must go, a complete tree must survive — and
# covers the interrupted-staging directory, a target path containing spaces, and
# a missing target directory.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/clean-incomplete-duckdb-unpack.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

RUNS=0
FAILED=0
ok() { RUNS=$((RUNS + 1)); echo "  ok: $1"; }
no() { RUNS=$((RUNS + 1)); FAILED=$((FAILED + 1)); echo "  FAIL: $1"; }

# A target path with a space proves the find/read loop is NUL-safe.
TARGET="${TEST_DIR}/target dir"
ROOT="${TARGET}/debug/build/libduckdb-sys-abc123/out/duckdb-sources"
COMPLETE="${ROOT}/aaaaaaaa"
INCOMPLETE="${ROOT}/bbbbbbbb"
STAGING="${ROOT}/.duckdb-unpack-42-0"
mkdir -p "${COMPLETE}" "${INCOMPLETE}" "${STAGING}"
: >"${COMPLETE}/.complete"
: >"${COMPLETE}/complete-source.cpp"
: >"${INCOMPLETE}/half-written.cpp"
: >"${STAGING}/partial.tar-entry"

output="$(bash "${SUBJECT}" "${TARGET}" 2>&1)"

if [[ -f "${COMPLETE}/.complete" ]] && [[ -f "${COMPLETE}/complete-source.cpp" ]]; then
    ok "a complete source tree is kept"
else
    no "a complete source tree was removed"
    cat <<<"${output}"
fi
if [[ ! -e "${INCOMPLETE}" ]]; then
    ok "an incomplete source tree is removed"
else
    no "an incomplete source tree survived"
fi
if [[ ! -e "${STAGING}" ]]; then
    ok "an abandoned staging directory is removed"
else
    no "an abandoned staging directory survived"
fi
if grep -qF "removed 1 incomplete source tree(s)" <<<"${output}"; then
    ok "the repair reports what it removed"
else
    no "the repair did not report the removal count"
    cat <<<"${output}"
fi
if grep -qF "duckdb unpack repair: removing incomplete source tree" <<<"${output}"; then
    ok "the repair names the incomplete tree"
else
    no "the repair did not name the incomplete tree"
    cat <<<"${output}"
fi

# A second run must be idempotent.
output="$(bash "${SUBJECT}" "${TARGET}" 2>&1)"
if grep -qF "removed 0 incomplete source tree(s)" <<<"${output}"; then
    ok "a repaired target is idempotent"
else
    no "a second repair run was not idempotent"
    cat <<<"${output}"
fi

# A complete-only tree must not be touched.
ONLY="${TEST_DIR}/only-complete"
mkdir -p "${ONLY}/debug/build/libduckdb-sys-def/out/duckdb-sources/cccccccc"
: >"${ONLY}/debug/build/libduckdb-sys-def/out/duckdb-sources/cccccccc/.complete"
bash "${SUBJECT}" "${ONLY}" >/dev/null 2>&1
if [[ -f "${ONLY}/debug/build/libduckdb-sys-def/out/duckdb-sources/cccccccc/.complete" ]]; then
    ok "a complete-only target is left intact"
else
    no "a complete-only target was damaged"
fi

# No target directory is a clean no-op, not an error.
status=0
bash "${SUBJECT}" "${TEST_DIR}/absent" >"${TEST_DIR}/absent.out" 2>&1 || status=$?
if [[ ${status} -eq 0 ]] && grep -qF "does not exist; nothing to clean" "${TEST_DIR}/absent.out"; then
    ok "a missing target directory is a clean no-op"
else
    no "a missing target directory was not handled (exit ${status})"
    cat "${TEST_DIR}/absent.out"
fi

echo
echo "clean-incomplete-duckdb-unpack fixture: ${RUNS} checks, ${FAILED} failure(s)"
if [[ ${FAILED} -ne 0 ]]; then
    exit 1
fi
