#!/usr/bin/env bash
# test-validate-skill-format.sh — negative fixtures for
# scripts/validate-skill-format.sh (ADR-0096: the SKILL.md 250-LOC cap,
# frontmatter, and local-reference gate).
#
# The gate is wired into scripts/validate.sh and the CI lint job, but until
# 2026-10-05 nothing tested *the gate itself*. Every other deterministic gate
# that mattered acquired a fixture after it passed for the wrong reason
# (scripts/test-llms-sync.sh, scripts/test-version-sync.sh,
# scripts/test-check-test-attributes.sh). This closes the last gap.
#
# The cases are chosen so a neuter cannot pass:
#   - the 250-line boundary is asserted on *both* sides, so `-gt` -> `-ge`,
#     `MAX_SKILL_LOC = 251`, or deleting the LOC check all fail here;
#   - an empty `.agents/skills` must fail, so "no files -> success" fails here;
#   - every failure case asserts the offender is *named* in the output, so a
#     gate that exits 1 without saying which skill passed nothing useful.
#
# Invoked by scripts/validate.sh (and therefore by the CI lint job).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

CHECK_STATUS=0
CHECK_OUTPUT=""

ESC="$(printf '\033')"

# run_gate — invoke the copied checker against the mock project root. The
# checker derives its own root from SCRIPT_DIR, so the copy is what isolates
# the fixture from the real 33 skills on disk. ANSI is stripped on capture
# because the checker wraps the offender marker in colour codes: a line-level
# "✗ <skill>" assertion written against raw output matches nothing at all and
# so passes vacuously, which is the failure mode this fixture exists to catch.
run_gate() {
    CHECK_STATUS=0
    CHECK_OUTPUT="$(bash "${TEST_DIR}/scripts/validate-skill-format.sh" 2>&1 \
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

# expect_fail_naming <label> <needle> — must exit non-zero AND name the offender.
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

# write_skill <dir-name> <body-file-contents>
write_skill() {
    local name="$1"
    mkdir -p "${TEST_DIR}/.agents/skills/${name}"
    printf '%s\n' "$2" > "${TEST_DIR}/.agents/skills/${name}/SKILL.md"
}

# skill_with_total_lines <dir-name> <total-lines> — frontmatter + padding, so the
# caller controls `wc -l` exactly and the boundary is not an approximation.
skill_with_total_lines() {
    local name="$1"
    local total="$2"
    local body_lines
    mkdir -p "${TEST_DIR}/.agents/skills/${name}"
    body_lines=$((total - 5))
    {
        printf -- '---\n'
        printf 'name: %s\n' "${name}"
        printf 'description: A fixture skill of exactly %d lines.\n' "${total}"
        printf -- '---\n'
        printf '# Title\n'
        seq 1 "${body_lines}" | sed 's/^/line /'
    } > "${TEST_DIR}/.agents/skills/${name}/SKILL.md"
}

reset_skills_dir() {
    rm -rf "${TEST_DIR}/.agents/skills"
    mkdir -p "${TEST_DIR}/.agents/skills" "${TEST_DIR}/scripts"
}

mkdir -p "${TEST_DIR}/scripts"
cp "${SCRIPT_DIR}/validate-skill-format.sh" "${TEST_DIR}/scripts/validate-skill-format.sh"
reset_skills_dir

echo "Test 0: an empty skills directory fails closed (never 'no files, all green')"
run_gate
expect_fail_naming "empty skills dir fails closed" "No SKILL.md files found"

write_skill good '---
name: good
description: When to use this skill.
---
# Good
Use it.'
echo "Test 1: a well-formed skill passes"
run_gate
expect_pass "well-formed skill"

echo "Test 2: exactly 250 lines passes (the cap is inclusive, not off-by-one)"
reset_skills_dir
skill_with_total_lines big 250
[[ "$(wc -l < "${TEST_DIR}/.agents/skills/big/SKILL.md")" -eq 250 ]] || {
    echo "❌ Fixture broken: expected 250 lines, got $(wc -l < "${TEST_DIR}/.agents/skills/big/SKILL.md")"
    exit 1
}
run_gate
expect_pass "250 lines == cap"

echo "Test 3: 251 lines fails and reports the measured count against the cap"
reset_skills_dir
skill_with_total_lines big 251
run_gate
expect_fail_naming "251 lines over cap" "251 lines (max 250)"

echo "Test 4: frontmatter name that does not match its directory fails"
reset_skills_dir
write_skill mismatch '---
name: some-other-name
description: Wrong name field.
---
# Mismatch'
run_gate
expect_fail_naming "name/directory mismatch" \
    "mismatch: name mismatch (frontmatter: 'some-other-name')"

echo "Test 5: missing description fails"
reset_skills_dir
write_skill nodesc '---
name: nodesc
---
# No description'
run_gate
expect_fail_naming "missing description" "missing or empty 'description' field"

echo "Test 6: no frontmatter delimiter fails and says so"
reset_skills_dir
write_skill nofm '# Just a heading

Some prose.'
run_gate
expect_fail_naming "missing frontmatter" "missing frontmatter (no --- at start)"

echo "Test 7: an unclosed frontmatter block fails (this used to validate — fail-open)"
reset_skills_dir
write_skill unclosed '---
name: unclosed
description: Never closed.'
run_gate
expect_fail_naming "unclosed frontmatter" \
    "unclosed: invalid frontmatter (no closing --- delimiter)"

echo "Test 7b: delimiters with nothing between them fail as an empty block"
reset_skills_dir
write_skill emptyfm '---
---
# Empty frontmatter body'
run_gate
expect_fail_naming "empty frontmatter block" \
    "emptyfm: empty frontmatter (no fields between the delimiters)"

echo "Test 8: a backticked skill-local path that does not exist fails"
reset_skills_dir
# shellcheck disable=SC2016  # the backticks are the gate's input, not substitutions
write_skill brokenref '---
name: brokenref
description: Points at a file that is not there.
---
# Broken

Read `references/gone.md` first.'
run_gate
expect_fail_naming "broken local reference" "brokenref: missing path \`references/gone.md\`"

echo "Test 9: the same reference resolves once the file exists (both directions)"
mkdir -p "${TEST_DIR}/.agents/skills/brokenref/references"
printf '# Gone\n' > "${TEST_DIR}/.agents/skills/brokenref/references/gone.md"
run_gate
expect_pass "reference restored"

echo "Test 10: a repo-root scripts/ path referenced from a skill resolves too"
reset_skills_dir
# shellcheck disable=SC2016  # the backticked script path must reach the gate literally
write_skill rootref '---
name: rootref
description: References a repo-root script.
---
# Root

Run `scripts/validate-skill-format.sh`.'
run_gate
expect_pass "repo-root scripts/ reference resolves"

echo "Test 11: URLs, anchors and @imports are not mistaken for local paths"
reset_skills_dir
# shellcheck disable=SC2016  # `@scope/package` must stay literal; double quotes would substitute
write_skill skipped '---
name: skipped
description: Contains non-path tokens.
---
# Skipped

See https://example.com/not/a/file.md, the @AGENTS.md import and [docs](#section).
Also `@scope/package` and `@0.3.7`.'
run_gate
expect_pass "URLs / anchors / @tokens skipped"

echo "Test 12: one bad skill among good ones still fails, and names only the bad one"
reset_skills_dir
write_skill alpha '---
name: alpha
description: First good skill.
---
# Alpha'
write_skill beta '---
name: typo-dir-name
description: Bad name field.
---
# Beta'
write_skill gamma '---
name: gamma
description: Third good skill.
---
# Gamma'
run_gate
expect_fail_naming "bad skill among good ones" \
    "beta: name mismatch (frontmatter: 'typo-dir-name')"
# Line-oriented: a valid skill must not share a report line with the offender
# marker. A whole-output substring test would match across lines, because the
# alpha "✓" line is followed somewhere later by beta's "✗" line.
if printf '%s\n' "${CHECK_OUTPUT}" | grep -E '✗ *(alpha|gamma)' >/dev/null 2>&1; then
    echo "❌ Failure (a valid skill was reported as broken)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (valid siblings not reported)"

echo "Test 13: the offender set is stable across runs (deterministic gate)"
reset_skills_dir
write_skill one '---
name: one
description: Ok.
---
# One'
write_skill two '---
name: wrong
description: Bad name.
---
# Two'
first="$(bash "${TEST_DIR}/scripts/validate-skill-format.sh" 2>&1 | grep -c '✗' || true)"
second="$(bash "${TEST_DIR}/scripts/validate-skill-format.sh" 2>&1 | grep -c '✗' || true)"
if [[ "${first}" == "${second}" && "${first}" -ge 1 ]]; then
    echo "✅ Success (${first} offender line(s), identical on both runs)"
else
    echo "❌ Failure (non-deterministic or zero offenders: ${first} vs ${second})"
    exit 1
fi

echo "All tests passed!"
