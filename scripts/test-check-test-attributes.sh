#!/usr/bin/env bash
# test-check-test-attributes.sh — negative fixtures for
# scripts/check-test-attributes.sh.
#
# Follows the precedent of scripts/test-llms-sync.sh / scripts/test-version-sync.sh:
# build an isolated mock tree in `mktemp -d` (no Cargo, no network), copy the
# checker in, then prove both directions — the shapes that must pass and the
# shape that must fail. The failing case is the real regression this gate exists
# for: a test body whose `#[test]` attribute was dropped, which compiles clean,
# lints clean and simply never runs.
#
# Invoked by scripts/validate.sh (and therefore by the CI lint job).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

CHECK_STATUS=0
CHECK_OUTPUT=""

# run_check <fixture-relative-path>... — the checker resolves the paths it is
# given, so fixtures live under a realistic `tests/…` layout and the sub-module
# exemption is exercised on real-looking paths.
run_check() {
    CHECK_STATUS=0
    CHECK_OUTPUT="$(cd "${TEST_DIR}" && bash "${TEST_DIR}/scripts/check-test-attributes.sh" "$@" 2>&1)" \
        || CHECK_STATUS=$?
}

expect_pass() {
    if [[ "${CHECK_STATUS}" -ne 0 ]]; then
        echo "❌ Failure (expected pass): $*"
        echo "${CHECK_OUTPUT}"
        exit 1
    fi
    echo "✅ Success ($*)"
}

expect_fail_naming() {
    local label="$1"
    local needle="$2"
    if [[ "${CHECK_STATUS}" -eq 0 ]]; then
        echo "❌ Failure (expected failure): ${label}"
        echo "${CHECK_OUTPUT}"
        exit 1
    fi
    if [[ "${CHECK_OUTPUT}" != *"${needle}"* ]]; then
        echo "❌ Failure (${label}: offender not named — wanted '${needle}')"
        echo "${CHECK_OUTPUT}"
        exit 1
    fi
    echo "✅ Success (${label})"
}

# count_offenders <fixture-relative-path>... — number of reported violations.
count_offenders() {
    local out=""
    out="$(cd "${TEST_DIR}"; bash "${TEST_DIR}/scripts/check-test-attributes.sh" "$@" 2>&1; exit 0)"
    printf '%s\n' "${out}" | grep -c 'missing test attribute' || true
}

mkdir -p "${TEST_DIR}/scripts" "${TEST_DIR}/tests/common"
cp "${SCRIPT_DIR}/check-test-attributes.sh" "${TEST_DIR}/scripts/check-test-attributes.sh"

# ── Fixtures ────────────────────────────────────────────────────────────────
cat > "${TEST_DIR}/tests/registered.rs" <<'RS'
#[test]
fn works() {}

#[tokio::test]
async fn runtime_test() {}

#[test_case(1, 2; "sum")]
#[test_case(3, 4; "other")]
fn parametrised(a: i32, b: i32) {
    let _ = (a, b);
}

#[test]
#[ignore = "flaky on CI"]
fn parked() {}

#[test]
#[should_panic]
fn panics_on_purpose() {
    panic!("expected");
}

// A doc comment above a plain helper must not be mistaken for an attribute.
/// Build the binary path.
fn helper() -> String {
    String::new()
}

#[test]
fn uses_helper() {
    let _ = helper();
}
RS

cat > "${TEST_DIR}/tests/dead_body.rs" <<'RS'
const EXPECTED: &[&str] = &["a"];

fn csm_bin() -> String {
    String::new()
}

#[test]
fn live_test() {
    let _ = csm_bin();
}

fn every_subcommand_has_help() {
    for cmd in EXPECTED {
        let _ = cmd;
    }
}
RS

cat > "${TEST_DIR}/tests/indented_module.rs" <<'RS'
mod tests {
    #[test]
    fn nested_registered() {}

    fn nested_unregistered_and_unreferenced() {}
}
RS

cat > "${TEST_DIR}/tests/common/mod.rs" <<'RS'
pub fn shared_fixture() -> String {
    String::new()
}
RS

cat > "${TEST_DIR}/tests/pub_top_level_dead.rs" <<'RS'
pub fn top_level_pub_and_unreferenced() {}
RS

echo "Test 1: registered attributes (test / tokio::test / test_case / ignore / should_panic) plus a referenced helper pass"
run_check tests/registered.rs
expect_pass "registered attributes + referenced helper"

echo "Test 2: the real regression — a test body with no #[test] and no call site fails and names the offender"
run_check tests/dead_body.rs
expect_fail_naming "missing #[test] detected" \
    "missing test attribute: tests/dead_body.rs:12 every_subcommand_has_help"

echo "Test 3: the same body passes once the attribute is restored"
# Rewritten verbatim (no sed insertion) so the two fixtures differ only by the
# `#[test]` line — the exact mutation this gate must detect in both directions.
cat > "${TEST_DIR}/tests/dead_body.rs" <<'RS'
const EXPECTED: &[&str] = &["a"];

fn csm_bin() -> String {
    String::new()
}

#[test]
fn live_test() {
    let _ = csm_bin();
}

#[test]
fn every_subcommand_has_help() {
    for cmd in EXPECTED {
        let _ = cmd;
    }
}
RS
run_check tests/dead_body.rs
expect_pass "restored #[test]"

echo "Test 4: a multi-line attribute block is read as one block"
cat > "${TEST_DIR}/tests/multiline_attr.rs" <<'RS'
#[test_case(
    1,
    2;
    "sum"
)]
fn parametrised_across_lines(a: i32, b: i32) {
    let _ = (a, b);
}
RS
run_check tests/multiline_attr.rs
expect_pass "multi-line #[test_case] block"

echo "Test 5: an fn inside a nested mod is a documented blind spot (not reported)"
run_check tests/indented_module.rs
expect_pass "nested mod (blind spot stays documented, not a failure)"

echo "Test 6: a pub fn in a tests/<dir>/ sub-module is exempt (cross-file call sites)"
run_check tests/common/mod.rs
expect_pass "tests/common/mod.rs pub helper"

echo "Test 7: a pub fn at tests root is still reported (nothing can import it)"
run_check tests/pub_top_level_dead.rs
expect_fail_naming "unimportable pub fn at tests root" \
    "missing test attribute: tests/pub_top_level_dead.rs:1 top_level_pub_and_unreferenced"

echo "Test 8: a missing input file fails explicitly and names the path"
run_check tests/does_not_exist.rs
if [[ "${CHECK_STATUS}" -eq 0 ]]; then
    echo "❌ Failure (missing input passed)"
    exit 1
fi
if [[ "${CHECK_OUTPUT}" != *"input missing"* ]]; then
    echo "❌ Failure (missing input not reported clearly)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (missing input named)"

echo "Test 9: the offender set is stable across runs (deterministic gate)"
first="$(cd "${TEST_DIR}" \
    && bash "${TEST_DIR}/scripts/check-test-attributes.sh" \
        tests/registered.rs tests/pub_top_level_dead.rs 2>&1 \
    | grep -c 'missing test attribute' || true)"
second="$(cd "${TEST_DIR}" \
    && bash "${TEST_DIR}/scripts/check-test-attributes.sh" \
        tests/registered.rs tests/pub_top_level_dead.rs 2>&1 \
    | grep -c 'missing test attribute' || true)"
if [[ "${first}" == "1" && "${first}" == "${second}" ]]; then
    echo "✅ Success (1 offender, identical on both runs)"
else
    echo "❌ Failure (non-deterministic or wrong count: ${first} vs ${second})"
    exit 1
fi

echo "All tests passed!"
