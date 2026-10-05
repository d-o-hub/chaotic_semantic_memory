#!/usr/bin/env bash
# test-validate-changelog.sh - Regression tests for scripts/validate-changelog.sh
#
# The checker reads Cargo.toml and CHANGELOG.md from the *current directory*, so every case
# builds a throwaway repo in a sandbox and runs the checker from inside it. Cases assert both
# the exit code and the message: a red that prints the wrong reason is a red nobody can act on.
#
# Run: bash scripts/test-validate-changelog.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="${SCRIPT_DIR}/validate-changelog.sh"

TOTAL=0
FAILED=0

new_repo() {
    REPO="$(mktemp -d)"
    cp "${CHECKER}" "${REPO}/validate-changelog.sh"
    chmod +x "${REPO}/validate-changelog.sh"
}

# repo_cargo <version> — the checker greps '^version =', so the spacing here is load-bearing
# and Test 7 pins it.
repo_cargo() {
    printf '[package]\nname = "chaotic_semantic_memory"\nversion = "%s"\n' "${1}" > "${REPO}/Cargo.toml"
}

run_check() {
    CHECK_STATUS=0
    CHECK_OUTPUT="$( cd "${REPO}" && ./validate-changelog.sh "${1:-}" 2>&1 )" || CHECK_STATUS=$?
}

expect_pass() {
    ((TOTAL++)) || true
    if [[ "${CHECK_STATUS}" -eq 0 ]] && printf '%s\n' "${CHECK_OUTPUT}" | grep -qF "$2"; then
        echo "✓ $1"
    else
        echo "✗ $1 (expected RC=0 and '${2}', got RC=${CHECK_STATUS})"
        printf '%s\n' "${CHECK_OUTPUT}" | sed 's/^/      | /'
        ((FAILED++)) || true
    fi
}

expect_fail() {
    ((TOTAL++)) || true
    if [[ "${CHECK_STATUS}" -ne 0 ]] && printf '%s\n' "${CHECK_OUTPUT}" | grep -qF "$2"; then
        echo "✓ $1 (RC=${CHECK_STATUS})"
    else
        echo "✗ $1 (expected a non-zero RC and '${2}', got RC=${CHECK_STATUS})"
        printf '%s\n' "${CHECK_OUTPUT}" | sed 's/^/      | /'
        ((FAILED++)) || true
    fi
}

expect_fail_not_mentioning() {
    # A per-section rule must not report a heading that is legal: the needle is the offender
    # line, and this asserts the offender set excludes the argument.
    local label="$1" offender="$2"
    ((TOTAL++)) || true
    if [[ "${CHECK_STATUS}" -ne 0 ]] && ! printf '%s\n' "${CHECK_OUTPUT}" | grep -qF "${offender}"; then
        echo "✓ ${label} (RC=${CHECK_STATUS}, '${offender}' not reported)"
    else
        echo "✗ ${label} (expected RC!=0 without reporting '${offender}', got RC=${CHECK_STATUS})"
        printf '%s\n' "${CHECK_OUTPUT}" | sed 's/^/      | /'
        ((FAILED++)) || true
    fi
}

echo "Running validate-changelog.sh regression tests"
echo ""

# --- Test 1: a clean changelog passes and names the version it validated ----------------
new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- one entry
- another entry

## [1.2.3] - 2026-07-28

### Added
- feature A

### Changed
- fix B
FIXTURE
run_check
expect_pass "Test 1: clean changelog passes" "validation passed for 1.2.3"

# --- Test 2: the real defect — a section reopened by a second '### Changed' --------------
# This is the exact shape that reached main: one '### Changed' under [Unreleased] became two.
new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- first block

### Changed
- second block that reopened the section

## [1.2.3] - 2026-07-28

### Added
- feature A
FIXTURE
run_check
expect_fail "Test 2: duplicate '### Changed' under [Unreleased] fails" "'## [Unreleased]' declares '### Changed' 2 times at lines: 5 8"

# --- Test 3: not tuned to the historical case — released sections score too --------------
new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- entry

## [1.2.3] - 2026-07-28

### Added
- feature A

### Added
- feature B duplicated
FIXTURE
run_check
expect_fail "Test 3: duplicate '### Added' inside a released version fails" "'## [1.2.3] - 2026-07-28' declares '### Added' 2 times"

# --- Test 4: the design claim — the rule is per section, not document-wide ---------------
# The same '### Changed' legitimately appears under every release. A markdownlint MD024-style
# document-wide checker reports this fixture; this case is what separates the two designs.
new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- upcoming

## [1.2.3] - 2026-07-28

### Changed
- released

## [1.2.2] - 2026-06-01

### Changed
- older release
FIXTURE
run_check
expect_pass "Test 4: the same heading under different versions is legal" "validation passed for 1.2.3"

# --- Test 5: the count and the line list are computed, not hardcoded --------------------
new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Fixed
- a

### Fixed
- b

### Fixed
- c

## [1.2.3] - 2026-07-28

### Added
- feature A
FIXTURE
run_check
expect_fail "Test 5: three copies report count 3 and all three lines" "'## [Unreleased]' declares '### Fixed' 3 times at lines: 5 8 11"

# --- Test 6: the other rules still fire (the new check did not displace them) -----------
new_repo
repo_cargo "9.9.9"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- entry

## [1.2.3] - 2026-07-28

### Added
- feature A
FIXTURE
run_check
expect_fail "Test 6: missing header for the current version fails" "No CHANGELOG entry for version 9.9.9"

new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- entry

## [1.2.3]

### Added
- feature A
FIXTURE
run_check
expect_fail "Test 6b: a version header without a date fails" "::error::"

# --- Test 7: the Cargo.toml coupling the checker depends on -----------------------------
# 'grep "^version ="' is space-sensitive. A lockfile-style 'version="1.2.3"' yields an empty
# VERSION, and the checker must say so rather than validate nothing.
new_repo
printf '[package]\nname = "chaotic_semantic_memory"\nversion="1.2.3"\n' > "${REPO}/Cargo.toml"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- entry

## [1.2.3] - 2026-07-28

### Added
- feature A
FIXTURE
run_check
expect_fail "Test 7: 'version=' without spaces cannot resolve a version" "Could not extract version from Cargo.toml"

# --- Test 8: a missing CHANGELOG.md fails loudly, not as an empty pass ------------------
new_repo
repo_cargo "1.2.3"
run_check
expect_fail "Test 8: missing CHANGELOG.md does not pass silently" "::error::"

# --- Test 9: the offender set is stable across runs ------------------------------------
new_repo
repo_cargo "1.2.3"
cat > "${REPO}/CHANGELOG.md" <<'FIXTURE'
# Changelog

## [Unreleased]

### Changed
- first

### Changed
- second

### Removed
- x

### Removed
- y

## [1.2.3] - 2026-07-28

### Added
- feature A
FIXTURE
run_check
FIRST_OUTPUT="${CHECK_OUTPUT}"
run_check
((TOTAL++)) || true
if [[ "${CHECK_STATUS}" -ne 0 ]] && [[ "${FIRST_OUTPUT}" == "${CHECK_OUTPUT}" ]]; then
    # Two offenders, both named, and neither legal heading dragged in.
    if printf '%s\n' "${CHECK_OUTPUT}" | grep -qF "'### Changed' 2 times" \
        && printf '%s\n' "${CHECK_OUTPUT}" | grep -qF "'### Removed' 2 times" \
        && ! printf '%s\n' "${CHECK_OUTPUT}" | grep -qF "'### Added'"; then
        echo "✓ Test 9: two offenders reported identically on both runs, '### Added' untouched"
    else
        echo "✗ Test 9: offender set is wrong"
        printf '%s\n' "${CHECK_OUTPUT}" | sed 's/^/      | /'
        ((FAILED++)) || true
    fi
else
    echo "✗ Test 9: output differs between runs (RC=${CHECK_STATUS})"
    ((FAILED++)) || true
fi

echo ""
echo "Tests run: ${TOTAL}, failed: ${FAILED}"

if [[ "${FAILED}" -gt 0 ]]; then
    echo "❌ ${FAILED} test(s) failed"
    exit 1
fi

echo "✅ All tests passed!"
exit 0
