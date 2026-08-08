#!/usr/bin/env bash
# =============================================================================
# validate-skills-tests.sh - Negative-fixture tests for validate-skill-format.sh
# =============================================================================
# Usage: ./scripts/validate-skills-tests.sh
#
# Fail-closed contract tests. Every fixture under scripts/skill-nonfixtures/
# is an intentionally broken skill; the validator MUST reject each one with a
# non-zero exit code. If any fixture passes validation, this harness exits
# non-zero. A single well-formed control skill must pass.
#
# Fixtures (scripts/skill-nonfixtures/):
#   bad-frontmatter       - missing 'description' field
#   unclosed-frontmatter  - frontmatter never closes (--- ... no closing ---)
#   overlength            - SKILL.md exceeds the 250-line limit
#   missing-refs          - references/ghost.md does not exist
#   name-mismatch         - frontmatter name != directory name
#   unreferenced-ref      - references/unused.md never mentioned in SKILL.md
#   failing-check         - check.sh exits 1 (must fail validation)
#   valid-skill           - control: must PASS every check
#
# The failing-check fixture's check.sh also proves the runner contract: it
# appends one line per invocation and this harness asserts exactly one line
# (the runner executed the script exactly once, not zero and not twice).
#
# Exit codes:
#   0 - Every broken fixture was rejected and the control fixture accepted
#   1 - A fixture passed validation that should have failed (or vice versa)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="${SCRIPT_DIR}/skill-nonfixtures"
VALIDATOR="${SCRIPT_DIR}/validate-skill-format.sh"

FAIL_COUNT=0
PASS_COUNT=0

# run_validator <mock_dir> -> exit status of the validator in that mock tree
run_validator() {
    local mock_dir="$1"
    (cd "${mock_dir}" && bash scripts/validate-skill-format.sh >/dev/null 2>&1)
}

# expect_validator_fail <fixture> : fixture MUST be rejected (exit != 0)
# When <fixture> is failing-check, the per-skill log it produced inside the
# mocked tree is copied to MOCK_RUNS_LOG for the exactly-once assertion.
expect_validator_fail() {
    local fixture="$1"
    local mock_dir
    mock_dir=$(mktemp -d)
    mkdir -p "${mock_dir}/scripts" "${mock_dir}/.agents/skills"
    cp "${VALIDATOR}" "${mock_dir}/scripts/"
    cp -r "${FIXTURES_DIR}/${fixture}" "${mock_dir}/.agents/skills/${fixture}"

    if run_validator "${mock_dir}"; then
        echo "❌ FAIL: fixture '${fixture}' PASSED validation (must fail)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    else
        echo "✅ PASS: fixture '${fixture}' correctly rejected"
        PASS_COUNT=$((PASS_COUNT + 1))
        if [[ "${fixture}" == "failing-check" ]]; then
            cp "${mock_dir}/.agents/skills/failing-check/runs.log" "${MOCK_RUNS_LOG}"
        fi
    fi
    rm -rf "${mock_dir}"
}

# expect_validator_pass <fixture> : exit code must be 0
expect_validator_pass() {
    local fixture="$1"
    local mock_dir
    mock_dir=$(mktemp -d)
    mkdir -p "${mock_dir}/scripts" "${mock_dir}/.agents/skills"
    cp "${VALIDATOR}" "${mock_dir}/scripts/"
    cp -r "${FIXTURES_DIR}/${fixture}" "${mock_dir}/.agents/skills/${fixture}"

    if run_validator "${mock_dir}"; then
        echo "✅ PASS: control fixture '${fixture}' accepted"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "❌ FAIL: control fixture '${fixture}' was rejected"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    rm -rf "${mock_dir}"
}

rm -f "${FIXTURES_DIR}/failing-check/runs.log"
MOCK_RUNS_LOG=$(mktemp)
rm -f "${MOCK_RUNS_LOG}"

echo "==> Rejecting broken fixtures (fail-closed)"
expect_validator_fail bad-frontmatter
expect_validator_fail unclosed-frontmatter
expect_validator_fail overlength
expect_validator_fail missing-refs
expect_validator_fail failing-check
expect_validator_fail name-mismatch
expect_validator_fail unreferenced-ref

echo ""
echo "==> Accepting the well-formed control fixture"
expect_validator_pass valid-skill

echo ""
echo "==> Runner executes each skill script exactly once"
if [[ -f "${MOCK_RUNS_LOG}" ]]; then
    RUN_COUNT=$(wc -l < "${MOCK_RUNS_LOG}")
    if [[ "${RUN_COUNT}" -eq 1 ]]; then
        echo "✅ PASS: failing-check/check.sh executed exactly once (${RUN_COUNT} run)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "❌ FAIL: failing-check/check.sh executed ${RUN_COUNT} times, expected 1"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    rm -f "${MOCK_RUNS_LOG}"
else
    echo "❌ FAIL: failing-check/check.sh never executed (no runs.log)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

echo ""
echo "================================"
echo "Skill validation tests: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"
if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    exit 1
fi
echo "All skills validation tests passed!"