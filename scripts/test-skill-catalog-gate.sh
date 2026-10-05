#!/usr/bin/env bash
# test-skill-catalog-gate.sh — negative fixtures for
# scripts/check-skill-catalog.sh (and the discovery errors of
# scripts/gen-skill-catalog.sh).
#
# Follows the precedent of scripts/test-check-test-attributes.sh /
# scripts/test-llms-sync.sh: build an isolated mock tree in `mktemp -d` (no
# Cargo, no network), copy the gate and the generator in, then prove BOTH
# directions — a catalog that matches the tree must pass, and every shape that
# must fail has to fail. The failing cases are the whole point: issue #828 found
# a hand-written catalog claiming "32 skills." against 33 on disk, and a gate
# that only ever passes proves nothing.
#
# Cases: synced passes; dropped row fails; count line lying fails; missing
# catalog fails; a skill directory without SKILL.md makes the generator error and
# the gate must propagate that failure instead of reporting success (fail-closed);
# front matter without `name` fails; a skill outside CATEGORY_SPEC still gets
# catalogued under `Unclassified`; a `|` in a description is escaped, not dropped.
#
# Invoked by scripts/validate.sh (and therefore by the CI lint job).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

readonly CATALOG_REL=".agents/skills/CATALOG.md"
readonly SKILLS="${TEST_DIR}/.agents/skills"

CHECK_STATUS=0
CHECK_OUTPUT=""

run_check() {
    CHECK_STATUS=0
    CHECK_OUTPUT="$(bash "${TEST_DIR}/scripts/check-skill-catalog.sh" 2>&1)" \
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
        echo "❌ Failure (${label}: reason not reported — wanted '${needle}')"
        echo "${CHECK_OUTPUT}"
        exit 1
    fi
    echo "✅ Success (${label})"
}

# write_skill <name> <description> — a SKILL.md shaped like the real ones.
write_skill() {
    local name="$1" desc="$2"
    mkdir -p "${SKILLS}/${name}"
    cat > "${SKILLS}/${name}/SKILL.md" <<EOF
---
name: ${name}
description: ${desc}
---

# ${name}
EOF
}

sync_catalog() {
    bash "${TEST_DIR}/scripts/gen-skill-catalog.sh" >/dev/null
}

mkdir -p "${TEST_DIR}/scripts"
cp "${SCRIPT_DIR}/gen-skill-catalog.sh" "${TEST_DIR}/scripts/gen-skill-catalog.sh"
cp "${SCRIPT_DIR}/check-skill-catalog.sh" "${TEST_DIR}/scripts/check-skill-catalog.sh"
chmod +x "${TEST_DIR}/scripts/gen-skill-catalog.sh" "${TEST_DIR}/scripts/check-skill-catalog.sh"

# Two skills; neither is in CATEGORY_SPEC, so both land in `Unclassified` — that
# is the documented fallback, and it keeps the fixtures independent of the real
# repository's section membership.
write_skill alpha "Does alpha things. Use when alpha is needed."
write_skill beta "Does beta things. Use when beta is needed."

echo "Test 1: catalog matching the tree passes"
sync_catalog
run_check
expect_pass "synced catalog"
if [[ "${CHECK_OUTPUT}" != *"2 skills"* ]]; then
    echo "❌ Failure (pass path did not report the skill count)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (pass path reports the skill count)"

echo "Test 2: a deleted row fails and names the catalog"
sync_catalog
# shellcheck disable=SC2016  # the backticks are the literal row key in CATALOG.md
grep -v '^| `beta`' "${SKILLS}/CATALOG.md" > "${TEST_DIR}/trimmed.md"
mv "${TEST_DIR}/trimmed.md" "${SKILLS}/CATALOG.md"
run_check
expect_fail_naming "deleted row detected" "stale generated file: ${CATALOG_REL}"

echo "Test 3: a count line that disagrees with the rows fails"
sync_catalog
sed 's/^2 skills\./3 skills./' "${SKILLS}/CATALOG.md" > "${TEST_DIR}/lied.md"
mv "${TEST_DIR}/lied.md" "${SKILLS}/CATALOG.md"
run_check
expect_fail_naming "bogus count line detected" "stale generated file: ${CATALOG_REL}"

echo "Test 4: missing catalog fails explicitly, not as an empty baseline"
sync_catalog
rm "${SKILLS}/CATALOG.md"
run_check
expect_fail_naming "missing catalog detected" "missing generated file: ${CATALOG_REL}"

echo "Test 5: skill directory without SKILL.md — generator errors, gate fails closed"
sync_catalog
mkdir -p "${SKILLS}/ghost"
run_check
expect_fail_naming "missing SKILL.md propagated" "generator failed"
if [[ "${CHECK_OUTPUT}" != *"skill directory without SKILL.md"* ]]; then
    echo "❌ Failure (generator did not name the offending directory)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (offending directory named by the generator)"
rmdir "${SKILLS}/ghost"

echo "Test 6: front matter without a name fails"
write_skill gamma "Gamma exists."
printf '%s\n' '---' 'description: no name here' '---' '' '# gamma' \
    > "${SKILLS}/gamma/SKILL.md"
run_check
expect_fail_naming "nameless front matter rejected" "generator failed"
if [[ "${CHECK_OUTPUT}" != *"no front-matter 'name:'"* ]]; then
    echo "❌ Failure (nameless skill not named)"
    echo "${CHECK_OUTPUT}"
    exit 1
fi
echo "✅ Success (nameless skill named)"
write_skill gamma "Gamma restored."

echo "Test 7: a skill outside CATEGORY_SPEC is still catalogued"
sync_catalog
run_check
expect_pass "synced after adding gamma"
# shellcheck disable=SC2016  # the backticks are the literal row key, not a substitution
if ! grep -q '^| `gamma`' "${SKILLS}/CATALOG.md"; then
    echo "❌ Failure (a skill on disk is absent from the generated catalog)"
    exit 1
fi
echo "✅ Success (new skill appears without hand-editing a row)"
if ! grep -q '^## Unclassified (3)$' "${SKILLS}/CATALOG.md"; then
    echo "❌ Failure (unclassified skills not grouped under a visible heading)"
    grep -n '^## ' "${SKILLS}/CATALOG.md"
    exit 1
fi
echo "✅ Success (Unclassified section makes the gap visible)"

echo "Test 8: a pipe in a description is escaped, not dropped"
write_skill delta "Splits on | when needed. Use for pipe safety."
sync_catalog
run_check
expect_pass "synced with an escaped pipe"
# -F: the cell is expected to contain a literal backslash-pipe, which BRE would
# otherwise read as alternation.
# shellcheck disable=SC2016  # backticks and `\|` are literal CATALOG.md cell content
if ! grep -qF '| `delta` | Splits on \| when needed. Use for pipe safety. |' \
    "${SKILLS}/CATALOG.md"; then
    echo "❌ Failure (pipe not escaped in the table cell)"
    grep -n 'delta' "${SKILLS}/CATALOG.md"
    exit 1
fi
echo "✅ Success (pipe escaped in the table cell)"

echo "All tests passed!"
