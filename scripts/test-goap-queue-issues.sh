#!/usr/bin/env bash
# test-goap-queue-issues.sh — negative fixtures for
# scripts/check-goap-queue-issues.sh (issue #851).
#
# Same shape as scripts/test-quality-gates.sh / scripts/test-validate-skill-format.sh:
# a throwaway mktemp tree, the subject copied in, the external tool stubbed, and
# every check asserted in BOTH directions. The gate talks to GitHub, so
# determinism is the whole problem: a ${TEST_DIR}/bin/gh stub answers the single
# tracker call from a canned fixture file, so no token, no network and no
# dependence on what issue #101 actually says.
#
# The cases are chosen so a neuter cannot pass: each check has a fixture that
# only fails while the implementing comparison is intact — prose that must NOT
# be read as an effect key, a CLOSED GOAP issue that must NOT be UNQUEUED, a
# count that must disagree in both directions, an empty parse that must not read
# as "all clear", and a tracker payload missing a queued issue that must not
# default to OPEN.
#
# The repository's real plans/ files are deliberately NOT asserted against here:
# that invocation needs the live tracker, so it is not deterministic — the wired
# gate in scripts/validate.sh owns it.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/check-goap-queue-issues.sh"

ORIG_PATH="${PATH}"

TEST_DIR="$(mktemp -d)"
trap 'rm -rf "${TEST_DIR}"' EXIT

mkdir -p "${TEST_DIR}/bin" "${TEST_DIR}/nogh" "${TEST_DIR}/plans" "${TEST_DIR}/scripts"
cp "${SUBJECT}" "${TEST_DIR}/scripts/check-goap-queue-issues.sh"

GH_CALL_LOG="${TEST_DIR}/gh-calls.log"
ISSUES_JSON="${TEST_DIR}/issues.json"

# Stub gh: logs its argv (so the "exactly one gh call" constraint is assertable),
# answers `auth status` with success and the tracker query with the canned
# payload. Anything else is a hard error, so a second gh call added to the gate
# shows up here as a failure instead of as a silent network dependency.
cat > "${TEST_DIR}/bin/gh" <<'STUB'
#!/usr/bin/env bash
{
    printf 'gh'
    printf ' %s' "$@"
    printf '\n'
} >> "${CSM_GH_LOG:?}"
case "$*" in
    *auth*status*)
        if [[ -n "${CSM_GH_AUTH_FAIL:-}" ]]; then
            exit 1
        fi
        exit 0
        ;;
    *issue*list*) cat "${CSM_GOAP_ISSUES_FILE:?}" ;;
    *)
        echo "STUB-GH-UNEXPECTED-ARGS: $*" >&2
        exit 9
        ;;
esac
STUB
chmod 755 "${TEST_DIR}/bin/gh"

# The "gh is not installed" case cannot be built by filtering PATH: the real gh
# here lives in /usr/bin, the same directory as awk/grep/sed/jq/sort. So the
# no-gh run gets a curated PATH holding exactly the binaries the gate invokes —
# and no gh in it.
for tool in awk bash grep jq sed sort dirname cat; do
    ln -s "$(command -v "${tool}")" "${TEST_DIR}/nogh/${tool}"
done

RUNS=0
FAILURES=0
CHECK_STATUS=0
CHECK_OUTPUT=""

ok() { RUNS=$((RUNS + 1)); echo "✅ Success ($1)"; }
ko() { RUNS=$((RUNS + 1)); FAILURES=$((FAILURES + 1)); echo "❌ Failure ($1)"; printf '%s\n' "${CHECK_OUTPUT}"; }

# set_issues <"number|STATE|title">... — the canned tracker payload
set_issues() {
    local out="[]"
    local arg number state title
    for arg in "$@"; do
        IFS='|' read -r number state title <<< "${arg}"
        out="$(printf '%s' "${out}" | jq --argjson n "${number}" --arg s "${state}" \
            --arg t "${title}" '. + [{number: $n, state: $s, title: $t}]')"
    done
    printf '%s' "${out}" > "${ISSUES_JSON}"
}

# action_block <name> <issue-number-or-empty> <effect keys space-separated>
action_block() {
    local name="$1" issue="$2" effects="$3"
    local -a effs=()
    local key
    if [[ -n "${effects}" ]]; then
        read -r -a effs <<< "${effects}"
    fi
    printf '  - name: %s\n' "${name}"
    if [[ -n "${issue}" ]]; then printf '    github_issue: "#%s"\n' "${issue}"; fi
    printf '    status: queued\n    preconditions: []\n    effects:\n'
    for key in "${effs[@]}"; do printf '      %s: true\n' "${key}"; done
    printf '    notes: >\n      Notes for %s, folded at six spaces.\n' "${name}"
}

# write_queue <block>... — an ACTIONS.md holding exactly these actions
write_queue() {
    local block
    {
        printf '# ACTIONS — Active GOAP Action Queue\n\nactions:\n'
        for block in "$@"; do printf '%s\n\n' "${block}"; done
    } > "${TEST_DIR}/plans/ACTIONS.md"
}

# write_state <queued_actions_count> <action_last_completed> <effect keys...>
# Effect keys are written `false`: an action that is still queued has not earned
# its effect yet, and the gate's STATE check reads a queued action whose effect
# world_state reports `true` as work that landed and was never removed.
# <action_last_completed> must therefore name an action that is NOT in the queue.
write_state() {
    local count="$1" last="$2"
    shift 2
    local key
    {
        printf '# GOAP World State\n\nworld_state:\n'
        printf '  project_initialized: true\n'
        for key in "$@"; do printf '  %s: false\n' "${key}"; done
        printf '  queued_actions_count: %s        # fixture, measured not asserted\n' "${count}"
        printf '  # Must remain the LAST key and appear exactly once.\n'
        printf '  action_last_completed: %s\n' "${last}"
    } > "${TEST_DIR}/plans/GOAP_STATE.md"
}

# Baseline: two queued actions, both OPEN on the tracker, both effects declared
# pending, queued_actions_count correct, and action_last_completed naming a
# removed action. The noise issues pin the negative directions — #103 is a CLOSED
# GOAP issue that is legitimately not queued, #104 is closed and not GOAP, #105
# is open and not GOAP. None of the three may be reported.
write_baseline() {
    write_queue \
        "$(action_block alpha_test 101 alpha_effect)" \
        "$(action_block beta_test 102 beta_effect)"
    write_state 2 zeta_removed_action alpha_effect beta_effect
    set_issues \
        "101|OPEN|GOAP: alpha_test — first queued action" \
        "102|OPEN|GOAP: beta_test — second queued action" \
        "103|CLOSED|GOAP: already landed and correctly unqueued" \
        "104|CLOSED|ci: unrelated closed issue" \
        "105|OPEN|ci: unrelated open issue, not a GOAP action"
}

# run_gate [gh-bin-dir] [path-base] [CSM_GOAP_QUEUE_REQUIRED] [auth-fail-flag]
run_gate() {
    local bin_dir="${1:-${TEST_DIR}/bin}"
    local path_base="${2:-${ORIG_PATH}}"
    local required="${3:-}"
    local auth_fail="${4:-}"
    CHECK_STATUS=0
    CHECK_OUTPUT=""
    : > "${GH_CALL_LOG}"
    CHECK_OUTPUT="$(
        CSM_GH_LOG="${GH_CALL_LOG}" \
        CSM_GOAP_ISSUES_FILE="${ISSUES_JSON}" \
        CSM_GOAP_QUEUE_REQUIRED="${required}" \
        CSM_GH_AUTH_FAIL="${auth_fail}" \
        CSM_PLANS_DIR="" \
        PATH="${bin_dir}:${path_base}" \
            bash "${TEST_DIR}/scripts/check-goap-queue-issues.sh" --repo-root "${TEST_DIR}" 2>&1
    )" || CHECK_STATUS=$?
}

# run_gate_plans <CSM_PLANS_DIR value> [repo-root-arg] — the CSM_PLANS_DIR hook
# and its precedence against --repo-root, which the plain run_gate cannot express
# (it always passes the flag and always clears the env var).
run_gate_plans() {
    local plans_dir="$1"
    local repo_root="${2:-}"
    local -a extra=()
    if [[ -n "${repo_root}" ]]; then
        extra=(--repo-root "${repo_root}")
    fi
    CHECK_STATUS=0
    CHECK_OUTPUT=""
    : > "${GH_CALL_LOG}"
    CHECK_OUTPUT="$(
        CSM_GH_LOG="${GH_CALL_LOG}" \
        CSM_GOAP_ISSUES_FILE="${ISSUES_JSON}" \
        CSM_PLANS_DIR="${plans_dir}" \
        PATH="${TEST_DIR}/bin:${ORIG_PATH}" \
            bash "${TEST_DIR}/scripts/check-goap-queue-issues.sh" "${extra[@]}" 2>&1
    )" || CHECK_STATUS=$?
}

expect_pass() {
    if [[ "${CHECK_STATUS}" -ne 0 ]]; then ko "$1 (expected exit 0, got ${CHECK_STATUS})"; else ok "$1"; fi
}

expect_fail() {
    if [[ "${CHECK_STATUS}" -eq 0 ]]; then ko "$1 (expected non-zero exit, got 0)"; else ok "$1"; fi
}

expect_status() {
    if [[ "${CHECK_STATUS}" -ne "$2" ]]; then ko "$1 (expected exit $2, got ${CHECK_STATUS})"; else ok "$1"; fi
}

# expect_fail_naming <label> <needle>... — non-zero AND every needle present, so
# a gate that exits 1 without naming the offender proves nothing.
expect_fail_naming() {
    local label="$1"
    shift
    if [[ "${CHECK_STATUS}" -eq 0 ]]; then
        ko "${label} (expected non-zero exit, got 0)"
        return
    fi
    local needle
    for needle in "$@"; do
        if [[ "${CHECK_OUTPUT}" != *"${needle}"* ]]; then
            ko "${label} (offender not named — wanted '${needle}')"
            return
        fi
    done
    ok "${label}"
}

expect_output_has() {
    if [[ "${CHECK_OUTPUT}" == *"$2"* ]]; then ok "$1"; else ko "$1 (output lacks '$2')"; fi
}

expect_output_lacks() {
    if [[ "${CHECK_OUTPUT}" != *"$2"* ]]; then ok "$1"; else ko "$1 (output must not contain '$2')"; fi
}

echo "Setting up mock repository in ${TEST_DIR}..."

echo "Test 1: a fully consistent queue exits 0"
write_baseline
run_gate
expect_pass "consistent queue"
expect_output_has "success line reports the reconciliation" "ok: GOAP queue reconciled"

echo "Test 2: exactly one gh issue-list call serves every tracker check"
write_baseline
run_gate
GH_LIST_CALLS="$(grep -c 'issue list' "${GH_CALL_LOG}" || true)"
GH_TOTAL_CALLS="$(grep -c '^gh ' "${GH_CALL_LOG}" || true)"
if [[ "${GH_LIST_CALLS}" -eq 1 ]] && [[ "${GH_TOTAL_CALLS}" -le 2 ]]; then
    ok "one issue-list call plus the auth probe (${GH_TOTAL_CALLS} gh call(s) total)"
else
    CHECK_OUTPUT="$(cat "${GH_CALL_LOG}")"
    ko "expected exactly one issue-list call, got ${GH_LIST_CALLS} of ${GH_TOTAL_CALLS}"
fi

echo "Test 3: STALE — a queued action whose issue is CLOSED fails and names it"
write_baseline
set_issues \
    "101|OPEN|GOAP: alpha_test — first queued action" \
    "102|CLOSED|GOAP: beta_test — landed, the queue never noticed"
run_gate
expect_fail_naming "STALE names action, issue and state" "STALE:" "beta_test" "#102" "CLOSED"
expect_output_lacks "STALE does not report the unaffected action" "STALE: action 'alpha_test'"

echo "Test 4: UNQUEUED — an OPEN GOAP issue with no queue entry fails"
write_baseline
write_queue "$(action_block alpha_test 101 alpha_effect)"
write_state 1 zeta_removed_action alpha_effect
run_gate
expect_fail_naming "UNQUEUED names the orphan issue" "UNQUEUED:" "#102"

echo "Test 5: UNQUEUED ignores CLOSED GOAP issues and open non-GOAP issues"
write_baseline
run_gate
expect_pass "baseline noise issues are not UNQUEUED"
expect_output_lacks "closed GOAP #103 is not reported" "#103"
expect_output_lacks "open non-GOAP #105 is not reported" "#105"

echo "Test 6: UNDECLARED — an action with no github_issue key fails"
write_baseline
write_queue \
    "$(action_block alpha_test 101 alpha_effect)" \
    "$(action_block beta_test "" beta_effect)"
run_gate
# The needle is the missing-key wording, not just the UNDECLARED tag: a gate that
# folds "no key at all" into "not an issue number" would still exit 1 and still
# name the action, and a tag-only assertion would not notice the collapse.
expect_fail_naming "UNDECLARED names the untracked action" \
    "UNDECLARED:" "beta_test" "has no github_issue: key"

echo "Test 7: STATE — action_last_completed must appear exactly once"
write_baseline
write_state 2 zeta_removed_action alpha_effect beta_effect
printf '  action_last_completed: an_earlier_snapshot\n' >> "${TEST_DIR}/plans/GOAP_STATE.md"
run_gate
expect_fail_naming "duplicated key fails" "action_last_completed appears 2"
write_baseline
printf '> prose mentions action_last_completed, which the header blockquote does too\n' \
    >> "${TEST_DIR}/plans/GOAP_STATE.md"
run_gate
expect_pass "an unindented prose mention is not counted as a key"

echo "Test 8: STATE — queued_actions_count must equal the queue, both directions"
write_baseline
write_state 3 zeta_removed_action alpha_effect beta_effect
run_gate
expect_fail_naming "count too high fails" "queued_actions_count is 3" "2 action(s)"
write_state 1 zeta_removed_action alpha_effect beta_effect
run_gate
expect_fail_naming "count too low fails" "queued_actions_count is 1"
write_state 2 zeta_removed_action alpha_effect beta_effect
run_gate
expect_pass "count matching the queue passes"

echo "Test 9: STATE — an effect world_state already reports true may not stay queued"
write_baseline
sed -i 's/^  beta_effect: false$/  beta_effect: true/' "${TEST_DIR}/plans/GOAP_STATE.md"
run_gate
expect_fail_naming "achieved effect names action and key" \
    "beta_effect" "beta_test" "already reports its effect"
write_baseline
run_gate
expect_pass "the same effect declared pending (false) passes"

echo "Test 9b: STATE — action_last_completed may not name a still-queued action"
write_baseline
write_state 2 alpha_test alpha_effect beta_effect
run_gate
expect_fail_naming "a completed action still in the queue fails" \
    "action_last_completed: alpha_test" "still queued"
write_baseline
run_gate
expect_pass "naming a removed action passes"

echo "Test 9c: STATE — an absent effect key is pending, not an error"
write_baseline
write_state 2 zeta_removed_action alpha_effect
run_gate
expect_pass "a queued effect nobody declared is not reported"
expect_output_lacks "absent effect key is not an error" "beta_effect"

echo "Test 10: notes prose at six spaces is NOT read as an effect key"
write_queue "$(action_block alpha_test 101 alpha_effect)"
{
    printf '  - name: gamma_test\n'
    printf '    github_issue: "#107"\n'
    printf '    status: queued\n'
    printf '    effects:\n'
    printf '      gamma_effect: true\n'
    printf '    notes: >\n'
    printf '      constrained: it must be a root-crate unit test, and\n'
    printf '      placement: is fixed by the mutation profile.\n'
} >> "${TEST_DIR}/plans/ACTIONS.md"
write_state 2 zeta_removed_action alpha_effect gamma_effect
set_issues \
    "101|OPEN|GOAP: alpha_test — first queued action" \
    "107|OPEN|GOAP: gamma_test — prose inside notes must not be an effect"
run_gate
expect_pass "folded-scalar prose is not mistaken for an effect key"
expect_output_lacks "prose key 'constrained' was not collected" "constrained"
expect_output_lacks "prose key 'placement' was not collected" "placement"

echo "Test 11: no gh on PATH — loud SKIP, tracker checks off, local checks fatal"
write_baseline
write_queue \
    "$(action_block alpha_test 101 alpha_effect)" \
    "$(action_block beta_test "" beta_effect)"
write_state 9 zeta_removed_action alpha_effect
run_gate "${TEST_DIR}/nogh" "${TEST_DIR}/nogh"
expect_fail "locally broken queue still exits 1 without gh"
expect_output_has "SKIP is loud, not a pass" "SKIP — NOT A PASS"
expect_output_has "UNDECLARED still ran" "UNDECLARED:"
expect_output_has "STATE still ran" "queued_actions_count is 9"
expect_output_lacks "no STALE verdict is invented without the tracker" "STALE:"
expect_output_lacks "no UNQUEUED verdict is invented without the tracker" "UNQUEUED:"

echo "Test 12: no gh + locally consistent queue exits 0 (skip is not fatal by default)"
write_baseline
run_gate "${TEST_DIR}/nogh" "${TEST_DIR}/nogh"
expect_pass "consistent queue without a tracker"
expect_output_has "SKIP line printed" "SKIP — NOT A PASS"

echo "Test 13: CSM_GOAP_QUEUE_REQUIRED=true makes the unreachable tracker fatal"
write_baseline
run_gate "${TEST_DIR}/nogh" "${TEST_DIR}/nogh" "true"
expect_fail_naming "required mode fails closed on an unreachable tracker" \
    "CSM_GOAP_QUEUE_REQUIRED" "SKIP — NOT A PASS"

echo "Test 13b: gh present but unauthenticated takes the same SKIP path"
write_baseline
run_gate "${TEST_DIR}/bin" "${ORIG_PATH}" "" "1"
expect_pass "unauthenticated tracker is a skip, not a failure, by default"
expect_output_has "SKIP names the auth failure" "gh is not authenticated"
expect_output_has "SKIP is loud" "SKIP — NOT A PASS"
run_gate "${TEST_DIR}/bin" "${ORIG_PATH}" "true" "1"
expect_fail_naming "required mode is fatal when gh is unauthenticated" \
    "CSM_GOAP_QUEUE_REQUIRED" "gh is not authenticated"

echo "Test 14: a queued issue missing from the tracker payload fails closed"
write_baseline
set_issues "101|OPEN|GOAP: alpha_test — first queued action"
run_gate
expect_fail_naming "a truncated payload is not read as OPEN" "STALE:" "#102" "returned no such issue"

echo "Test 15: a queue that parses to zero actions is an error, not all-clear"
write_baseline
printf '# ACTIONS\n\nactions: []\n' > "${TEST_DIR}/plans/ACTIONS.md"
run_gate
expect_fail_naming "empty parse fails closed" "parsed 0 actions"
printf '    - name: indented_wrong\n      github_issue: "#101"\n' > "${TEST_DIR}/plans/ACTIONS.md"
run_gate
expect_fail_naming "a wrong indentation anchor parses to zero actions" "parsed 0 actions"

echo "Test 16: missing plans files fail closed"
write_baseline
rm -f "${TEST_DIR}/plans/ACTIONS.md"
run_gate
expect_fail_naming "missing ACTIONS.md" "input missing" "ACTIONS.md"
write_baseline
rm -f "${TEST_DIR}/plans/GOAP_STATE.md"
run_gate
expect_fail_naming "missing GOAP_STATE.md" "input missing" "GOAP_STATE.md"

echo "Test 17: bad invocation exits 2, not 1 or 0"
write_baseline
CHECK_STATUS=0
CHECK_OUTPUT="$(
    CSM_GH_LOG="${GH_CALL_LOG}" CSM_GOAP_ISSUES_FILE="${ISSUES_JSON}" \
    PATH="${TEST_DIR}/bin:${ORIG_PATH}" \
        bash "${TEST_DIR}/scripts/check-goap-queue-issues.sh" --bogus 2>&1
)" || CHECK_STATUS=$?
expect_status "unknown argument is a usage error" 2

echo "Test 18: the offender set is stable across runs (deterministic gate)"
write_baseline
write_queue "$(action_block alpha_test 101 alpha_effect)"
write_state 1 zeta_removed_action alpha_effect
set_issues "101|CLOSED|GOAP: alpha_test — landed" "106|OPEN|GOAP: gamma_test — filed not queued"
first="$(
    CSM_GH_LOG="${GH_CALL_LOG}" CSM_GOAP_ISSUES_FILE="${ISSUES_JSON}" \
        PATH="${TEST_DIR}/bin:${ORIG_PATH}" \
        bash "${TEST_DIR}/scripts/check-goap-queue-issues.sh" --repo-root "${TEST_DIR}" 2>&1 |
        grep -cE 'STALE:|UNQUEUED:' || true
)"
second="$(
    CSM_GH_LOG="${GH_CALL_LOG}" CSM_GOAP_ISSUES_FILE="${ISSUES_JSON}" \
        PATH="${TEST_DIR}/bin:${ORIG_PATH}" \
        bash "${TEST_DIR}/scripts/check-goap-queue-issues.sh" --repo-root "${TEST_DIR}" 2>&1 |
        grep -cE 'STALE:|UNQUEUED:' || true
)"
if [[ "${first}" == "${second}" ]] && [[ "${first}" -ge 2 ]]; then
    ok "${first} offender line(s), identical on both runs"
else
    ko "non-deterministic or under-reported offender set: ${first} vs ${second}"
fi

echo "Test 19: CSM_PLANS_DIR is honored, and --repo-root outranks it"
# Measured 2026-10-05: an exported CSM_PLANS_DIR used to re-point every sandboxed
# fixture run at the caller's real plans tree (12 assertions red), so the
# precedence is now pinned rather than assumed.
write_baseline
mkdir -p "${TEST_DIR}/otherplans"
cp "${TEST_DIR}/plans/ACTIONS.md" "${TEST_DIR}/plans/GOAP_STATE.md" "${TEST_DIR}/otherplans/"
rm -f "${TEST_DIR}/plans/GOAP_STATE.md"
run_gate_plans "${TEST_DIR}/otherplans"
expect_pass "CSM_PLANS_DIR alone selects the alternate plans tree"
run_gate_plans "${TEST_DIR}/otherplans" "${TEST_DIR}"
expect_fail_naming "--repo-root outranks an ambient CSM_PLANS_DIR" "input missing" "GOAP_STATE.md"

echo "────────────────────────────────────────────────────────"
echo "Tests run: ${RUNS}, failed: ${FAILURES}"
if [[ "${FAILURES}" -ne 0 ]]; then
    exit 1
fi
echo "All tests passed!"
