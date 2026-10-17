#!/usr/bin/env bash
# test-check-audit-ignore-parity.sh — synthetic cases for issue #903's
# cargo-deny / cargo-audit ignore parity checker. Every input lives under a
# throwaway directory; the repository's real policy files are never edited.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check-audit-ignore-parity.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

RUNS=0
FAILURES=0
CHECK_RC=0
CHECK_STDOUT=""
CHECK_STDERR=""

pass() {
    RUNS=$((RUNS + 1))
    printf '✅ %s\n' "$1"
}

fail() {
    RUNS=$((RUNS + 1))
    FAILURES=$((FAILURES + 1))
    printf '❌ %s\n' "$1" >&2
}

run_checker() {
    CHECK_RC=0
    CHECK_STDOUT=""
    CHECK_STDERR=""
    CHECK_STDOUT="$(bash "${SUBJECT}" "$@" 2>"${TEST_DIR}/stderr.log")" || CHECK_RC=$?
    CHECK_STDERR="$(cat "${TEST_DIR}/stderr.log")"
}

write_ids() {
    local file="$1"
    shift
    {
        printf '%s\n' '[advisories]' 'ignore = ['
        for id in "$@"; do
            printf '    "%s",\n' "${id}"
        done
        printf '%s\n' ']'
    } > "${file}"
}

expect_pass() {
    if [[ "${CHECK_RC}" -eq 0 ]]; then
        pass "$1"
    else
        fail "$1 (expected rc 0, got ${CHECK_RC})"
        printf '%s\n' "${CHECK_STDOUT}" "${CHECK_STDERR}" >&2
    fi
}

expect_fail_naming() {
    local label="$1"
    local needle
    shift
    local missing=0

    if [[ "${CHECK_RC}" -ne 1 ]]; then
        fail "${label} (expected rc 1, got ${CHECK_RC})"
        missing=1
    fi
    for needle in "$@"; do
        if [[ "${CHECK_STDERR}" != *"${needle}"* ]]; then
            fail "${label} (stderr lacks '${needle}')"
            missing=1
        fi
    done
    if [[ "${missing}" -eq 0 ]]; then
        pass "$label"
    fi
}

expect_stdout_has() {
    if [[ "${CHECK_STDOUT}" == *"$2"* ]]; then
        pass "$1"
    else
        fail "$1 (stdout lacks '$2')"
    fi
}

printf '%s\n' 'Test 1: equal advisory sets pass'
write_ids "${TEST_DIR}/deny.toml" RUSTSEC-2099-0001 RUSTSEC-2099-0002
write_ids "${TEST_DIR}/audit.toml" RUSTSEC-2099-0002 RUSTSEC-2099-0001
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"
expect_pass 'equal sets exit 0'
expect_stdout_has 'success reports the unique ID count' 'match (2 ids)'

printf '%s\n' 'Test 2: deny-only advisory fails and is named'
write_ids "${TEST_DIR}/deny.toml" RUSTSEC-2099-0001 RUSTSEC-2099-0002
write_ids "${TEST_DIR}/audit.toml" RUSTSEC-2099-0001
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"
expect_fail_naming 'deny-only ID is named with both file paths' \
    'RUSTSEC-2099-0002' "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"

printf '%s\n' 'Test 3: audit-only advisory fails and is named'
write_ids "${TEST_DIR}/deny.toml" RUSTSEC-2099-0001
write_ids "${TEST_DIR}/audit.toml" RUSTSEC-2099-0001 RUSTSEC-2099-0003
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"
expect_fail_naming 'audit-only ID is named with both file paths' \
    'RUSTSEC-2099-0003' "${TEST_DIR}/audit.toml" "${TEST_DIR}/deny.toml"

printf '%s\n' 'Test 4: a missing file fails closed'
write_ids "${TEST_DIR}/deny.toml" RUSTSEC-2099-0001
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/missing.toml"
expect_fail_naming 'missing file names the failed input' \
    'no RUSTSEC ids parsed from' "${TEST_DIR}/missing.toml"

printf '%s\n' 'Test 5: a file with zero IDs fails closed'
printf '%s\n' '[advisories]' 'ignore = []' > "${TEST_DIR}/deny.toml"
write_ids "${TEST_DIR}/audit.toml" RUSTSEC-2099-0001
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"
expect_fail_naming 'zero-ID file is not read as parity' \
    'no RUSTSEC ids parsed from' "${TEST_DIR}/deny.toml"

printf '%s\n' 'Test 6: comments and IDs outside [advisories] are ignored'
cat > "${TEST_DIR}/deny.toml" <<'TOML'
[advisories]
ignore = ["RUSTSEC-2099-0001"] # RUSTSEC-2099-0002
# RUSTSEC-2099-0003
[licenses]
ignore = ["RUSTSEC-2099-0004"]
TOML
cat > "${TEST_DIR}/audit.toml" <<'TOML'
[advisories]
ignore = ["RUSTSEC-2099-0001"]
[licenses]
ignore = ["RUSTSEC-2099-0005"]
TOML
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"
expect_pass 'commented and out-of-section IDs do not affect parity'
expect_stdout_has 'only one active advisory ID is counted' 'match (1 ids)'

printf '%s\n' 'Test 7: a repeated [advisories] block cannot hide an extra ID'
cat > "${TEST_DIR}/deny.toml" <<'TOML'
[advisories]
ignore = ["RUSTSEC-2099-0001"]
TOML
cat > "${TEST_DIR}/audit.toml" <<'TOML'
[advisories]
ignore = ["RUSTSEC-2099-0001"]
[advisories]
ignore = ["RUSTSEC-2099-0006"]
TOML
run_checker "${TEST_DIR}/deny.toml" "${TEST_DIR}/audit.toml"
expect_fail_naming 'the second advisory block ID is detected' \
    'RUSTSEC-2099-0006' "${TEST_DIR}/audit.toml" "${TEST_DIR}/deny.toml"

printf '%s\n' "Tests run: ${RUNS}, failed: ${FAILURES}"
if [[ "${FAILURES}" -ne 0 ]]; then
    exit 1
fi
printf '%s\n' 'All tests passed!'
