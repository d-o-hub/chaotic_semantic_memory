#!/usr/bin/env bash
# scripts/test-gate-failure-detail.sh — deterministic cases for issue #960's
# refusal diagnostic. The real gate only fails on a main release attempt, so
# this fixture makes no network calls: every case feeds a recorded
# GET /actions/runs/{id}/jobs payload on stdin and asserts the printed SUMMARY.
#
# Each case is a control in one direction; case 4 and case 6 are the negative
# controls that keep the script from blaming somebody when nobody failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/gate-failure-detail.sh"

RUNS=0
FAILURES=0

pass() {
    RUNS=$((RUNS + 1))
    printf '✅ %s\n' "$1"
}

fail() {
    RUNS=$((RUNS + 1))
    FAILURES=$((FAILURES + 1))
    printf '❌ %s\n' "$1" >&2
}

OUT=""
RC=0

# run_subject <stdin-file> [args...] — captures stdout+rc, never aborts the suite.
run_subject() {
    local stdin_file="$1"
    shift
    RC=0
    OUT=""
    if OUT="$(bash "${SUBJECT}" "$@" < "${stdin_file}" 2>&1)"; then
        RC=0
    else
        RC=$?
    fi
}

summary_line() {
    printf '%s\n' "${OUT}" | sed -n 's/^SUMMARY: //p'
}

expect_rc() {
    if [[ "${RC}" -eq "$1" ]]; then
        pass "$2 (rc ${RC})"
    else
        fail "$2 — expected rc $1, got ${RC}"
        printf '%s\n' "${OUT}" | sed 's/^/      | /' >&2
    fi
}

expect_contains() {
    if summary_line | grep -qF -- "$1"; then
        pass "$2 contains '$1'"
    else
        fail "$2 — SUMMARY lacks '$1'"
        printf '      SUMMARY: %s\n' "$(summary_line)" >&2
    fi
}

expect_absent() {
    if ! printf '%s' "${OUT}" | grep -qF -- "$1"; then
        pass "$2 does not contain '$1'"
    else
        fail "$2 — unexpected '$1' in output"
        printf '%s\n' "${OUT}" | sed 's/^/      | /' >&2
    fi
}

CASES="$(mktemp -d)"
trap 'rm -rf "${CASES}"' EXIT

# 1. The #960 case: an open PR blocks the queue policy. Release must name it.
cat > "${CASES}/drain_red.json" <<'JSON'
{"total_count": 7, "jobs": [
 {"name":"pr-drain-check","status":"completed","conclusion":"failure"},
 {"name":"version-triad","status":"completed","conclusion":"success"},
 {"name":"security-audit","status":"completed","conclusion":"success"},
 {"name":"wasm-smoke-build","status":"completed","conclusion":"success"},
 {"name":"planning-state-check","status":"completed","conclusion":"success"},
 {"name":"benchmark-gate","status":"completed","conclusion":"success"},
 {"name":"gate-summary","status":"completed","conclusion":"failure"}
]}
JSON
run_subject "${CASES}/drain_red.json" 41078875116 4ac8422dbd82
expect_rc 0 "case 1 drain-red"
expect_contains "2 of 7 gate jobs failed" "case 1 drain-red"
expect_contains "pr-drain-check" "case 1 drain-red"
expect_contains "gate-summary" "case 1 drain-red"

# 2. The jobless run: a rejected workflow file, not a failed check. This is the
#    signature that hid release.yml for eight days (0f77c0d0 → #947).
cat > "${CASES}/jobless.json" <<'JSON'
{"total_count": 0, "jobs": []}
JSON
run_subject "${CASES}/jobless.json" 41070030431 ae117fb9
expect_rc 0 "case 2 jobless"
expect_contains "ZERO jobs" "case 2 jobless"

# 3. The #953 timeout: a sibling still running when the deadline expires.
cat > "${CASES}/in_flight.json" <<'JSON'
{"total_count": 3, "jobs": [
 {"name":"pr-drain-check","status":"completed","conclusion":"success"},
 {"name":"security-audit","status":"in_progress","conclusion":null},
 {"name":"gate-summary","status":"queued","conclusion":null}
]}
JSON
run_subject "${CASES}/in_flight.json" 39648046307 11111111
expect_rc 0 "case 3 in-flight"
expect_contains "no verdict yet" "case 3 in-flight"
expect_contains "security-audit(in_progress)" "case 3 in-flight"

# 4. Negative control — nothing failed, so nothing may be accused.
cat > "${CASES}/all_green.json" <<'JSON'
{"total_count": 2, "jobs": [
 {"name":"version-triad","status":"completed","conclusion":"success"},
 {"name":"benchmark-gate","status":"completed","conclusion":"success"}
]}
JSON
run_subject "${CASES}/all_green.json" 1 2
expect_rc 0 "case 4 all-green"
expect_contains "verdict disagrees with its jobs" "case 4 all-green"
expect_absent "failed:" "case 4 all-green"

# 5. The #914 class: skipped needs under a failed run, no explicit failure.
cat > "${CASES}/skipped.json" <<'JSON'
{"total_count": 2, "jobs": [
 {"name":"benchmark-gate","status":"completed","conclusion":"skipped"},
 {"name":"security-audit","status":"completed","conclusion":"success"}
]}
JSON
run_subject "${CASES}/skipped.json" 1 2
expect_rc 0 "case 5 skipped"
expect_contains "skipped" "case 5 skipped"
expect_contains "#914" "case 5 skipped"

# 6. Unusable payload must degrade to a stated inability, not a crash and not a
#    silent nothing (exit 0: the caller already holds the refusal verdict).
printf 'not json at all\n' > "${CASES}/garbage.txt"
run_subject "${CASES}/garbage.txt" 1 2
expect_rc 0 "case 6 garbage-in"
expect_contains "detail unavailable" "case 6 garbage-in"

# 7. Log injection: a job name carrying a newline plus a forged annotation.
#    Output must stay one line per job + one SUMMARY, and contain no '::'.
printf '{"total_count":1,"jobs":[{"name":"evil\\n::error::forged\\njob","status":"completed","conclusion":"failure"}]}\n' \
    > "${CASES}/inject.json"
run_subject "${CASES}/inject.json" 1 2
expect_rc 0 "case 7 inject"
LINES="$(printf '%s\n' "${OUT}" | grep -c '')"
if [[ "${LINES}" -eq 2 ]]; then
    pass "case 7 inject holds output to 2 lines"
else
    fail "case 7 inject — expected 2 output lines, got ${LINES}"
    printf '%s\n' "${OUT}" | sed 's/^/      | /' >&2
fi
expect_absent "::error::" "case 7 inject"
expect_contains "1 of 1 gate jobs failed" "case 7 inject"

# 8. Caller misuse is the only non-zero exit.
RC=0
if OUT="$(bash "${SUBJECT}" 1 < /dev/null 2>&1)"; then
    RC=0
else
    RC=$?
fi
expect_rc 2 "case 8 missing argument"

printf '\n%d checks, %d failures\n' "${RUNS}" "${FAILURES}"
if [[ "${FAILURES}" -gt 0 ]]; then
    exit 1
fi
exit 0
