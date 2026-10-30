#!/usr/bin/env bash
# test-pre-release-gate-summary.sh — deterministic cases for issue #903's
# extracted pre-release gate verdict. The real gate only runs on a main CI
# completion, so this fixture makes no network calls and writes summaries only
# under a temporary directory.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/pre-release-gate-summary.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

RUNS=0
FAILURES=0
CHECK_RC=0
CHECK_STDOUT=""
CHECK_STDERR=""
CURRENT_SUMMARY=""

pass() {
    RUNS=$((RUNS + 1))
    printf '✅ %s\n' "$1"
}

fail() {
    RUNS=$((RUNS + 1))
    FAILURES=$((FAILURES + 1))
    printf '❌ %s\n' "$1" >&2
}

run_cli() {
    CHECK_RC=0
    CHECK_STDOUT=""
    CHECK_STDERR=""
    CHECK_STDOUT="$(bash "${SUBJECT}" "$@" 2>"${TEST_DIR}/stderr.log")" || CHECK_RC=$?
    CHECK_STDERR="$(cat "${TEST_DIR}/stderr.log")"
}

run_summary() {
    CURRENT_SUMMARY="$1"
    shift
    : > "${CURRENT_SUMMARY}"
    run_cli "${CURRENT_SUMMARY}" "$@"
}

expect_pass() {
    if [[ "${CHECK_RC}" -eq 0 ]]; then
        pass "$1"
    else
        fail "$1 (expected rc 0, got ${CHECK_RC})"
        printf '%s\n' "${CHECK_STDOUT}" "${CHECK_STDERR}" >&2
    fi
}

expect_status() {
    if [[ "${CHECK_RC}" -eq "$2" ]]; then
        pass "$1"
    else
        fail "$1 (expected rc $2, got ${CHECK_RC})"
        printf '%s\n' "${CHECK_STDOUT}" "${CHECK_STDERR}" >&2
    fi
}

expect_fail_naming() {
    local label="$1"
    local expected_rc="$2"
    shift 2
    local needle
    local missing=0

    if [[ "${CHECK_RC}" -ne "${expected_rc}" ]]; then
        fail "${label} (expected rc ${expected_rc}, got ${CHECK_RC})"
        missing=1
    fi
    for needle in "$@"; do
        if [[ "${CHECK_STDERR}" != *"${needle}"* ]]; then
            fail "${label} (stderr lacks '${needle}')"
            missing=1
        fi
    done
    if [[ "${missing}" -eq 0 ]]; then
        pass "${label}"
    fi
}

expect_file_has() {
    if grep -qF -- "$2" "${CURRENT_SUMMARY}"; then
        pass "$1"
    else
        fail "$1 (summary lacks '$2')"
    fi
}

expect_file_lacks() {
    if grep -qF -- "$2" "${CURRENT_SUMMARY}"; then
        fail "$1 (summary unexpectedly contains '$2')"
    else
        pass "$1"
    fi
}

expect_stdout_has() {
    if [[ "${CHECK_STDOUT}" == *"$2"* ]]; then
        pass "$1"
    else
        fail "$1 (stdout lacks '$2')"
    fi
}

expect_stderr_has() {
    if [[ "${CHECK_STDERR}" == *"$2"* ]]; then
        pass "$1"
    else
        fail "$1 (stderr lacks '$2')"
    fi
}

count_data_rows() {
    local count
    count="$(grep -cE '^\| [^|]+ \| (success|failure|cancelled|skipped) \|$' "$1" || true)"
    printf '%s' "${count}"
}

expect_row_count() {
    local actual
    actual="$(count_data_rows "${CURRENT_SUMMARY}")"
    if [[ "${actual}" -eq "$2" ]]; then
        pass "$1"
    else
        fail "$1 (expected $2 data rows, got ${actual})"
    fi
}

GATE_NAMES=(
    'PR Drain=skipped'
    'Version Triad=skipped'
    'Security Audit=skipped'
    'WASM Build=skipped'
    'Planning State=skipped'
    'Benchmark Gate=skipped'
)

printf '%s\n' 'Test 1: failed CI with six skipped checks has no gate verdict'
run_summary "${TEST_DIR}/no-verdict.md" failure deadbeef "${GATE_NAMES[@]}"
expect_pass 'failure conclusion exits 0'
expect_file_has 'summary reports no verdict with trigger and SHA' "⏭️ **No verdict** — CI concluded \`failure\` for \`deadbeef\`"
expect_row_count 'summary contains all six blocking check rows' 6
expect_stdout_has 'stdout emits the GitHub warning annotation' '::warning title=Pre-release gate::No verdict:'
expect_file_lacks 'no-verdict summary does not report a gate failure' 'Gate failed'

printf '%s\n' 'Test 2: six successful blocking checks pass'
run_summary "${TEST_DIR}/pass.md" success deadbeef \
    'PR Drain=success' 'Version Triad=success' 'Security Audit=success' \
    'WASM Build=success' 'Planning State=success' 'Benchmark Gate=success'
expect_pass 'successful checks exit 0'
expect_file_has 'summary reports all blocking gates passed' '✅ **All blocking gates passed**'

printf '%s\n' 'Test 3: a failed check fails the gate and is named'
run_summary "${TEST_DIR}/failed-check.md" success deadbeef 'Security Audit=failure'
expect_status 'failed check exits 1' 1
expect_file_has 'summary reports gate failure' 'Gate failed'
expect_file_has 'summary names failed check' 'Failed checks: Security Audit'

printf '%s\n' 'Test 4: skipped need on successful CI is not a pass'
run_summary "${TEST_DIR}/skipped-check.md" success deadbeef 'Benchmark Gate=skipped'
expect_status 'skipped check on successful CI exits 1' 1
expect_file_has 'summary names skipped check as failed' 'Failed checks: Benchmark Gate'

printf '%s\n' 'Test 5: no arguments are a usage error'
run_cli
expect_fail_naming 'missing arguments exit 2 and print usage' 2 'Usage:'

printf '%s\n' 'Test 6: unknown trigger conclusion is rejected'
run_summary "${TEST_DIR}/unknown-trigger.md" banana deadbeef 'PR Drain=success'
expect_fail_naming 'unknown trigger exits 2 with conclusion named' 2 \
    "unexpected workflow_run conclusion 'banana'" 'cannot compute a verdict'

printf '%s\n' 'Test 7: unknown result is rejected and names its pair'
run_summary "${TEST_DIR}/unknown-result.md" success deadbeef 'PR Drain=banana'
expect_fail_naming 'unknown result exits 2 with pair named' 2 'PR Drain' 'banana'

printf '%s\n' 'Test 8: summary row count matches the number of check pairs'
run_summary "${TEST_DIR}/row-count.md" success deadbeef 'Alpha=success' 'Beta=success'
expect_pass 'two checks pass'
expect_row_count 'exactly two data rows are emitted for two pairs' 2

printf '%s\n' "Tests run: ${RUNS}, failed: ${FAILURES}"
if [[ "${FAILURES}" -ne 0 ]]; then
    exit 1
fi
printf '%s\n' 'All tests passed!'
