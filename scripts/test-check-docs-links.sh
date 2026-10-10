#!/usr/bin/env bash
# test-check-docs-links.sh — negative fixtures for scripts/check-docs-links.sh
# (link, command, and version consistency validator).
#
# Asserts both directions of every check:
#   - valid @ imports, relative links, and script paths pass;
#   - broken @ imports, broken relative links, missing script paths, and
#     version mismatches fail and name the offending file;
#   - excluded paths (plans/.archive/**, .mimocode/**) and version tags / SHAs
#     / example syntax are skipped;
#   - deterministic reporting across repeated runs.

# shellcheck disable=SC2016  # fixture doc strings contain literal backticks (code fences, @scope/pkg)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

CHECK_STATUS=0
CHECK_OUTPUT=""

ESC="$(printf '\033')"

run_gate() {
    CHECK_STATUS=0
    CHECK_OUTPUT="$(cd "${TEST_DIR}" && bash "${TEST_DIR}/scripts/check-docs-links.sh" 2>&1 \
        | sed -e "s/${ESC}\[[0-9;]*m//g")" \
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
        echo "❌ Failure (expected non-zero exit): ${label}"
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

setup_baseline_tree() {
    rm -rf "${TEST_DIR:?}"/* "${TEST_DIR:?}"/.[!.]* "${TEST_DIR:?}"/..?* 2>/dev/null || true
    mkdir -p "${TEST_DIR}/scripts" "${TEST_DIR}/wasm" "${TEST_DIR}/cli-npm" "${TEST_DIR}/book/src"

    cp "${SCRIPT_DIR}/check-docs-links.sh" "${TEST_DIR}/scripts/check-docs-links.sh"

    # Core package files
    printf 'version = "0.3.8"\n' > "${TEST_DIR}/Cargo.toml"
    printf '0.3.8\n' > "${TEST_DIR}/VERSION"
    printf '{"version": "0.3.8"}\n' > "${TEST_DIR}/wasm/package.json"
    printf '{"version": "0.3.8"}\n' > "${TEST_DIR}/cli-npm/package.json"
    printf 'name = "chaotic_semantic_memory"\nversion = "0.3.8"\n' > "${TEST_DIR}/Cargo.lock"

    # Documentation files
    printf 'chaotic_semantic_memory = { version = "0.3" }\n' > "${TEST_DIR}/README.md"
    printf 'chaotic_semantic_memory = { version = "0.3" }\n' > "${TEST_DIR}/book/src/getting-started.md"
    printf '## [0.3.8] - 2026-10-05\n' > "${TEST_DIR}/CHANGELOG.md"
    printf 'version 0.3.8\n' > "${TEST_DIR}/llms.txt"
    printf 'version 0.3.8\n' > "${TEST_DIR}/llms-full.txt"

    # Root referenced files
    printf '# AGENTS\n' > "${TEST_DIR}/AGENTS.md"
}

setup_baseline_tree

echo "Test 1: baseline well-formed tree passes"
run_gate
expect_pass "baseline tree"

echo "Test 2: valid root-relative @ import and relative link pass"
mkdir -p "${TEST_DIR}/docs"
printf '# Target\n' > "${TEST_DIR}/docs/target.md"
printf '# Doc\nSee @AGENTS.md and [target](./target.md).\n' > "${TEST_DIR}/docs/doc.md"
run_gate
expect_pass "valid @ import and relative link"

echo "Test 3: broken @ import fails and names offender"
printf '# Doc\nSee @NONEXISTENT.md\n' > "${TEST_DIR}/docs/bad_at.md"
run_gate
expect_fail_naming "broken @ import" "docs/bad_at.md: broken link '@NONEXISTENT.md'"
rm "${TEST_DIR}/docs/bad_at.md"

echo "Test 4: broken relative link fails and names offender"
printf '# Doc\nSee [missing](./missing.md)\n' > "${TEST_DIR}/docs/bad_rel.md"
run_gate
expect_fail_naming "broken relative link" "docs/bad_rel.md: broken link './missing.md'"
rm "${TEST_DIR}/docs/bad_rel.md"

echo "Test 5: missing script in code block fails and names offender"
printf '# Doc\n```bash\n./scripts/nonexistent.sh\n```\n' > "${TEST_DIR}/docs/bad_cmd.md"
run_gate
expect_fail_naming "missing code block script" "docs/bad_cmd.md: script 'scripts/nonexistent.sh' not found"
rm "${TEST_DIR}/docs/bad_cmd.md"

echo "Test 6: version mismatch in wasm/package.json fails and names file"
printf '{"version": "0.3.7"}\n' > "${TEST_DIR}/wasm/package.json"
run_gate
expect_fail_naming "version mismatch in wasm/package.json" "wasm/package.json: 0.3.7 (expected 0.3.8)"
printf '{"version": "0.3.8"}\n' > "${TEST_DIR}/wasm/package.json"

echo "Test 7: archived and mimocode directories are excluded from link checking"
mkdir -p "${TEST_DIR}/plans/.archive/historical" "${TEST_DIR}/.mimocode/plans"
printf '# Archived\nSee [broken](./gone.md) and @GONE.md\n' > "${TEST_DIR}/plans/.archive/historical/old.md"
printf '# Mimocode\nSee [broken](./gone.md) and @0.3.7\n' > "${TEST_DIR}/.mimocode/plans/plan.md"
run_gate
expect_pass "archived and mimocode excluded"

echo "Test 8: version tags, SHAs, scope packages, and example syntax are skipped"
printf '# Doc\nSee @0.3.7, @v1.0.0, @0631aa6515c7d545823c67cfae7ef4fc7f490154, `@scope/pkg`, @file.md and [text](./path.md).\n' > "${TEST_DIR}/docs/examples.md"
run_gate
expect_pass "version tags, SHAs, and example syntax skipped"

echo "Test 9: deterministic error reporting on repeated runs"
printf '# Doc\nSee @MISSING_ONE.md\n' > "${TEST_DIR}/docs/err1.md"
printf '# Doc\nSee @MISSING_TWO.md\n' > "${TEST_DIR}/docs/err2.md"
run_gate
first_count="$(echo "${CHECK_OUTPUT}" | grep -c 'broken link' || true)"
run_gate
second_count="$(echo "${CHECK_OUTPUT}" | grep -c 'broken link' || true)"
if [[ "${first_count}" -eq "${second_count}" && "${first_count}" -eq 2 ]]; then
    echo "✅ Success (deterministic output: ${first_count} broken links on both runs)"
else
    echo "❌ Failure (non-deterministic output: ${first_count} vs ${second_count})"
    exit 1
fi

echo "All tests passed!"
