#!/usr/bin/env bash
# test-ci-workspace-crate-matrix.sh — negative fixtures for
# scripts/ci-workspace-crate-matrix.sh (issue #827).
#
# Follows the precedent of scripts/test-llms-sync.sh and
# scripts/test-check-test-attributes.sh: build an isolated mock tree in
# `mktemp -d`, copy the checker in, and drive it through the test hooks
# (CSM_CARGO_METADATA_JSON / CSM_CI_WORKFLOW / CSM_MATRIX_EXCLUSIONS) so no real
# cargo, no network and no workflow edit is needed.
#
# Every invariant is proven in BOTH directions — the passing shape and the
# failing shape. A divergence gate that has only ever been seen green is not
# evidence (progress/LEARNINGS.md 2026-09-30).
#
# Invoked by scripts/validate.sh, so the CI `lint` job runs it.

# GitHub Actions expressions (${{ ... }}) are literal fixture content and must
# never be expanded by the shell.
# shellcheck disable=SC2016

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

command -v jq >/dev/null 2>&1 || { echo "jq is required to run this fixture" >&2; exit 1; }

CHECKER="${TEST_DIR}/scripts/ci-workspace-crate-matrix.sh"
META="${TEST_DIR}/metadata.json"
WORKFLOW="${TEST_DIR}/.github/workflows/ci.yml"
mkdir -p "${TEST_DIR}/scripts" "${TEST_DIR}/.github/workflows"
cp "${SCRIPT_DIR}/ci-workspace-crate-matrix.sh" "${CHECKER}"

CHECK_STATUS=0
CHECK_OUTPUT=""
FIXTURE_EXCLUSIONS="csm-gamma"

# run_check [mode-flag...] — the fixture env is always exported for these runs.
run_check() {
    CHECK_STATUS=0
    CHECK_OUTPUT="$(CSM_CARGO_METADATA_JSON="${META}" CSM_CI_WORKFLOW="${WORKFLOW}" \
        CSM_MATRIX_EXCLUSIONS="${FIXTURE_EXCLUSIONS}" \
        bash "${CHECKER}" "$@" 2>&1)" || CHECK_STATUS=$?
}

# run_list — stdout of list mode, for the determinism comparison.
run_list() {
    CSM_CARGO_METADATA_JSON="${META}" CSM_CI_WORKFLOW="${WORKFLOW}" \
        CSM_MATRIX_EXCLUSIONS="${FIXTURE_EXCLUSIONS}" bash "${CHECKER}"
}

expect_pass() {
    if [[ "${CHECK_STATUS}" -ne 0 ]]; then
        echo "❌ Failure (expected pass): $1"
        echo "${CHECK_OUTPUT}"
        exit 1
    fi
    echo "✅ Success ($1)"
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

# write_metadata CRATE... — one `crates/<name>` package per argument, plus a
# root crate and an out-of-tree `benchmarks` member that must never be derived.
write_metadata() {
    jq -n --arg wr "${TEST_DIR}" '$ARGS.positional
        | {workspace_root: $wr,
           packages: ([{name: "chaotic_semantic_memory", manifest_path: ($wr + "/Cargo.toml")},
                      {name: "do-chaotic-semantic-memory-bench", manifest_path: ($wr + "/benchmarks/Cargo.toml")}]
                      + [.[] | {name: ., manifest_path: ($wr + "/crates/" + . + "/Cargo.toml")}])}' \
        --args "$@" > "${META}"
}

# write_workflow — the matrix job consumes the derived output, and `csm-gamma`
# has the dedicated job that makes excluding it legal.
write_workflow() {
    cat > "${WORKFLOW}" <<'YML'
name: Fixture CI
on: pull_request
jobs:
  workspace-matrix:
    runs-on: ubuntu-latest
    steps:
    - run: bash scripts/ci-workspace-crate-matrix.sh --github-output
  test-workspace-crates:
    name: Test Workspace Crates
    needs: workspace-matrix
    strategy:
      fail-fast: false
      matrix: ${{ fromJSON(needs.workspace-matrix.outputs.crates) }}
    steps:
    - run: cargo test -p ${{ matrix.crate }} --locked
  test-gamma:
    name: Test csm-gamma (dedicated job)
    steps:
    - run: cargo test -p csm-gamma --locked --all-features
YML
}

write_metadata csm-alpha csm-beta csm-gamma
write_workflow

echo "Test 1: baseline — candidates minus a proven exclusion, in the documented JSON shape"
run_check --json
expect_pass "baseline derivation"
if [[ "${CHECK_OUTPUT}" != '{"crate":["csm-alpha","csm-beta"]}' ]]; then
    echo "❌ Failure (wrong matrix JSON): got '${CHECK_OUTPUT}'"
    exit 1
fi
echo '✅ Success (JSON is exactly {"crate":["csm-alpha","csm-beta"]})'

echo "Test 2: the root crate and the out-of-tree benchmarks member are not derived"
run_check
expect_pass "candidate filter"
if [[ "${CHECK_OUTPUT}" == *"chaotic_semantic_memory"* || "${CHECK_OUTPUT}" == *"do-chaotic-semantic-memory-bench"* ]]; then
    echo "❌ Failure (non-crates/* member leaked into the matrix)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (only crates/* members are candidates)"

echo "Test 3: THE REGRESSION THIS GATE KILLS — a newly added crates/* member lands"
echo "        in the matrix automatically, with no list anywhere to remember"
write_metadata csm-alpha csm-beta csm-delta csm-gamma
run_check
expect_pass "new member derived"
if [[ "${CHECK_OUTPUT}" != *"csm-delta"* ]]; then
    echo "❌ Failure (csm-delta missing from the derived matrix)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (csm-delta tested without touching any hand-written entry)"

echo "Test 4: an exclusion is only legal while its dedicated job exists — deleting"
echo "        the job turns the exclusion back into the silent skip"
printf '%s\n' \
    'name: Fixture CI' 'jobs:' \
    '  test-workspace-crates:' \
    '    strategy:' \
    '      matrix: ${{ fromJSON(needs.workspace-matrix.outputs.crates) }}' \
    '    steps:' \
    '      - run: cargo test -p ${{ matrix.crate }} --locked' > "${WORKFLOW}"
run_check --check
expect_fail_naming "exclusion without a dedicated job fails" \
    "excluded crate has no dedicated job in ci.yml: csm-gamma"

echo "Test 5: restoring the job makes the same crate legal to exclude again"
write_workflow
run_check --check
expect_pass "dedicated job restored"

echo "Test 6: a prose comment naming the crate is not coverage (comments are stripped)"
printf '%s\n' \
    'name: Fixture CI' 'jobs:' \
    '  test-workspace-crates:' \
    '    strategy:' \
    '      matrix: ${{ fromJSON(needs.workspace-matrix.outputs.crates) }}' \
    '    steps:' \
    '      - run: cargo test -p ${{ matrix.crate }} --locked' \
    '  docs-only:' \
    '    steps:' \
    '      # csm-gamma is covered by cargo test -p csm-gamma, promised in prose' \
    '      - run: echo nothing tested' > "${WORKFLOW}"
run_check --check
expect_fail_naming "comment-only mention is not coverage" \
    "excluded crate has no dedicated job in ci.yml: csm-gamma"
write_workflow

echo "Test 7: an empty derived matrix fails instead of shipping zero green jobs"
write_metadata csm-gamma
run_check --json
expect_fail_naming "empty matrix rejected" "the derived matrix is empty"

echo "Test 8: an exclusion whose crate left the workspace is stale and must fail"
write_metadata csm-alpha csm-beta csm-gamma
FIXTURE_EXCLUSIONS="csm-gamma csm-gone"
run_check --check
expect_fail_naming "stale exclusion rejected" \
    "excluded crate is no longer a workspace member: csm-gone"
FIXTURE_EXCLUSIONS="csm-gamma"

echo "Test 9: hand-written crate entries re-added to the matrix job fail the gate"
printf '%s\n' \
    'name: Fixture CI' 'jobs:' \
    '  test-workspace-crates:' \
    '    strategy:' \
    '      fail-fast: false' \
    '      matrix: ${{ fromJSON(needs.workspace-matrix.outputs.crates) }}' \
    '      # someone re-typed the old list during a quick fix:' \
    '        crate:' \
    '          - csm-alpha' \
    '          - csm-beta' \
    '    steps:' \
    '      - run: cargo test -p ${{ matrix.crate }} --locked' \
    '  test-gamma:' \
    '    steps:' \
    '      - run: cargo test -p csm-gamma --locked' > "${WORKFLOW}"
run_check --check
expect_fail_naming "re-typed list rejected" "hand-written crate entries are back"

echo "Test 10: a matrix that stopped using fromJSON fails too — the derive job may"
echo "         not become decorative"
printf '%s\n' \
    'name: Fixture CI' 'jobs:' \
    '  test-workspace-crates:' \
    '    strategy:' \
    '      matrix:' \
    '        crate: [csm-alpha, csm-beta]' \
    '    steps:' \
    '      - run: cargo test -p ${{ matrix.crate }} --locked' \
    '  test-gamma:' \
    '    steps:' \
    '      - run: cargo test -p csm-gamma --locked' > "${WORKFLOW}"
run_check --check
expect_fail_naming "non-derived matrix rejected" "no longer derives its matrix"

echo "Test 11: a workflow without the matrix job at all fails (nobody consumes it)"
printf '%s\n' 'name: Fixture CI' 'jobs:' \
    '  something-else:' '    steps:' \
    '      - run: cargo test -p csm-gamma --locked' > "${WORKFLOW}"
run_check --check
expect_fail_naming "missing matrix job rejected" "job 'test-workspace-crates' not found"

echo "Test 12: a missing workflow file is an explicit error, not a silent pass"
mv "${WORKFLOW}" "${WORKFLOW}.bak"
run_check --check
expect_fail_naming "missing workflow named" "workflow not found"
rm -f "${WORKFLOW}.bak"

echo "Test 13: garbage where metadata should be fails loudly"
printf 'not json at all\n' > "${META}"
run_check --json
expect_fail_naming "invalid metadata rejected" "no .packages array"

echo "Test 14: --github-output writes exactly one crates= line that parses back to"
echo "         what fromJSON() will receive"
write_workflow
write_metadata csm-alpha csm-beta csm-gamma
export GITHUB_OUTPUT="${TEST_DIR}/github_output.txt"
: > "${GITHUB_OUTPUT}"
run_check --github-output
expect_pass "github-output mode"
if [[ "$(grep -c '^crates=' "${GITHUB_OUTPUT}")" != "1" ]]; then
    echo "❌ Failure (GITHUB_OUTPUT must hold exactly one crates= line)"
    cat "${GITHUB_OUTPUT}"
    exit 1
fi
roundtrip="$(grep '^crates=' "${GITHUB_OUTPUT}" | sed 's/^crates=//')"
if [[ "${roundtrip}" != '{"crate":["csm-alpha","csm-beta"]}' ]]; then
    echo "❌ Failure (output value mismatch): ${roundtrip}"
    exit 1
fi
if ! printf '%s' "${roundtrip}" | jq -e '.crate | (type == "array") and (length == 2)' >/dev/null; then
    echo "❌ Failure (output value is not a fromJSON-able matrix object)"
    exit 1
fi
echo "✅ Success (GITHUB_OUTPUT value parses as the same 2-crate matrix)"

echo "Test 15: --github-output without GITHUB_OUTPUT fails instead of emitting nothing"
unset GITHUB_OUTPUT
run_check --github-output
expect_fail_naming "missing GITHUB_OUTPUT rejected" "needs GITHUB_OUTPUT"

echo "Test 16: the gate is deterministic — identical output on repeated runs"
first="$(run_list)"
second="$(run_list)"
if [[ -n "${first}" && "${first}" == "${second}" ]]; then
    echo "✅ Success (identical list on both runs)"
else
    echo "❌ Failure (non-deterministic or empty: '${first}' vs '${second}')"
    exit 1
fi

echo "All tests passed!"
