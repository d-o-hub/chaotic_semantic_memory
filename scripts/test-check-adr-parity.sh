#!/usr/bin/env bash
# Regression fixtures for scripts/check-adr-parity.sh.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="${SCRIPT_DIR}/check-adr-parity.sh"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "${TEST_ROOT}"' EXIT

TOTAL=0
FAILED=0

new_fixture() {
    rm -rf "${TEST_ROOT}/plans" "${TEST_ROOT}/docs"
    mkdir -p "${TEST_ROOT}/scripts" "${TEST_ROOT}/plans/adr" "${TEST_ROOT}/docs/adr"
    cp "${CHECKER}" "${TEST_ROOT}/scripts/check-adr-parity.sh"
    chmod 755 "${TEST_ROOT}/scripts/check-adr-parity.sh"
}

run_fixture() {
    STATUS=0
    OUTPUT="$(cd "${TEST_ROOT}" && ./scripts/check-adr-parity.sh 2>&1)" || STATUS=$?
}

expect_pass() {
    TOTAL=$((TOTAL + 1))
    if [[ "${STATUS}" -eq 0 ]] && [[ "${OUTPUT}" == *"$2"* ]]; then
        printf '✓ %s\n' "$1"
    else
        printf '✗ %s (expected RC=0 and %s, got RC=%s)\n' "$1" "$2" "${STATUS}"
        printf '%s\n' "${OUTPUT}" | sed 's/^/      | /'
        FAILED=$((FAILED + 1))
    fi
}

expect_fail() {
    TOTAL=$((TOTAL + 1))
    if [[ "${STATUS}" -ne 0 ]] && [[ "${OUTPUT}" == *"$2"* ]]; then
        printf '✓ %s (RC=%s)\n' "$1" "${STATUS}"
    else
        printf '✗ %s (expected RC!=0 and %s, got RC=%s)\n' "$1" "$2" "${STATUS}"
        printf '%s\n' "${OUTPUT}" | sed 's/^/      | /'
        FAILED=$((FAILED + 1))
    fi
}

# The real regression: mawk must recognize the intentional Superseded/N/A row.
new_fixture
printf '# ADR\n' > "${TEST_ROOT}/plans/adr/0001-one.md"
cat > "${TEST_ROOT}/plans/ADR_REGISTRY.md" <<'EOF'
| ADR | Title | Status | File |
|-----|-------|--------|------|
| 0001 | One | Accepted | [adr/0001-one.md](adr/0001-one.md) |
| 0003 | Superseded | Superseded | N/A |
EOF
run_fixture
expect_pass "superseded N/A rows are exempt" "registry=2, disk=1"

# A real missing accepted ADR must still fail closed.
new_fixture
cat > "${TEST_ROOT}/plans/ADR_REGISTRY.md" <<'EOF'
| ADR | Title | Status | File |
|-----|-------|--------|------|
| 0001 | One | Accepted | [adr/0001-one.md](adr/0001-one.md) |
EOF
run_fixture
expect_fail "missing accepted ADRs fail" "ADR-0001"

# The id space is not bounded to 00xx: a missing ADR-0100 must fail closed
# too.  A `00[0-9]{2}`-only pattern ignored every id above 0099.
new_fixture
cat > "${TEST_ROOT}/plans/ADR_REGISTRY.md" <<'EOF'
| ADR | Title | Status | File |
|-----|-------|--------|------|
| 0100 | Hundred | Accepted | [adr/0100-hundred.md](adr/0100-hundred.md) |
EOF
run_fixture
expect_fail "missing four-digit ADRs fail" "ADR-0100"

# The same id must pass when its file exists.
new_fixture
printf '# ADR\n' > "${TEST_ROOT}/plans/adr/0100-hundred.md"
cat > "${TEST_ROOT}/plans/ADR_REGISTRY.md" <<'EOF'
| ADR | Title | Status | File |
|-----|-------|--------|------|
| 0100 | Hundred | Accepted | [adr/0100-hundred.md](adr/0100-hundred.md) |
EOF
run_fixture
expect_pass "present four-digit ADRs pass" "registry=1, disk=1"
# IDs eventually become unpadded at 1000; the checker must cover that end of
# the four-digit space too.
new_fixture
cat > "${TEST_ROOT}/plans/ADR_REGISTRY.md" <<'EOF'
| ADR | Title | Status | File |
|-----|-------|--------|------|
| 1000 | Thousand | Accepted | [adr/1000-thousand.md](adr/1000-thousand.md) |
EOF
run_fixture
expect_fail "missing unpadded four-digit ADRs fail" "ADR-1000"

new_fixture
printf '# ADR\n' > "${TEST_ROOT}/plans/adr/1000-thousand.md"
cat > "${TEST_ROOT}/plans/ADR_REGISTRY.md" <<'EOF'
| ADR | Title | Status | File |
|-----|-------|--------|------|
| 1000 | Thousand | Accepted | [adr/1000-thousand.md](adr/1000-thousand.md) |
EOF
run_fixture
expect_pass "present unpadded four-digit ADRs pass" "registry=1, disk=1"

printf '\nADR parity fixture: %s total, %s failed\n' "${TOTAL}" "${FAILED}"
[[ "${FAILED}" -eq 0 ]]

