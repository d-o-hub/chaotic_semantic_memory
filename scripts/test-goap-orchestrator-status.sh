#!/usr/bin/env bash
# Regression fixture for the read-only status parser in goap-orchestrator.sh.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "${TEST_ROOT}"' EXIT
mkdir -p "${TEST_ROOT}/scripts" "${TEST_ROOT}/plans" "${TEST_ROOT}/bin"
cp "${SCRIPT_DIR}/goap-orchestrator.sh" "${TEST_ROOT}/scripts/goap-orchestrator.sh"
chmod 755 "${TEST_ROOT}/scripts/goap-orchestrator.sh"

# The status command should remain useful in an offline checkout.
cat > "${TEST_ROOT}/bin/gh" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
chmod 755 "${TEST_ROOT}/bin/gh"

cat > "${TEST_ROOT}/plans/GOAP_STATE.md" <<'EOF'
world_state:
  active_wave: 33 # comment with: a colon
  action_last_completed: completed_action # historical detail: ignored
EOF
cat > "${TEST_ROOT}/plans/ACTIONS.md" <<'EOF'
# ACTIONS
> Historical prose: status: queued and status: in_progress are not actions.
actions:
  - name: live_action
    status: queued
  - name: running_action
    status: in_progress
EOF

OUTPUT="$(PATH="${TEST_ROOT}/bin:${PATH}" "${TEST_ROOT}/scripts/goap-orchestrator.sh" status 2>&1)"
CLEAN_OUTPUT="$(printf '%s\n' "${OUTPUT}" | sed $'s/\033\\[[0-9;]*m//g')"
for expected in \
    "Last completed: completed_action" \
    "Queued actions: 1" \
    "In-progress: 1" \
    "Active wave: 33"; do
    if [[ "${CLEAN_OUTPUT}" != *"${expected}"* ]]; then
        printf 'missing expected status line: %s\n%s\n' "${expected}" "${CLEAN_OUTPUT}" >&2
        exit 1
    fi
done

printf 'GOAP status parser fixture: passed\n'

# ────────────────────────────────────────────────────────────────────────────
# ci_truth_on_main_tip: the verdict must be derived from the push run whose head
# equals origin/main, and a still-running job reports an EMPTY conclusion. That
# empty field is the regression this covers: reading the row with tab as IFS made
# `read` collapse it and shift run_id into conclusion (observed as `run= `).
# ────────────────────────────────────────────────────────────────────────────
REPO2="${TEST_ROOT}/repo2"
mkdir -p "${REPO2}/scripts" "${REPO2}/plans"
cp "${SCRIPT_DIR}/goap-orchestrator.sh" "${REPO2}/scripts/goap-orchestrator.sh"
chmod 755 "${REPO2}/scripts/goap-orchestrator.sh"
printf 'world_state:\n  active_wave: 33\n  action_last_completed: fixture_action\n' > "${REPO2}/plans/GOAP_STATE.md"
printf '# ACTIONS\nactions:\n' > "${REPO2}/plans/ACTIONS.md"
git -C "${REPO2}" init -q
git -C "${REPO2}" config user.email fixture@example.invalid
git -C "${REPO2}" config user.name fixture
: > "${REPO2}/README"
git -C "${REPO2}" add README
git -C "${REPO2}" commit -qm fixture
git -C "${REPO2}" update-ref refs/remotes/origin/main HEAD
TIP=$(git -C "${REPO2}" rev-parse --short=8 origin/main)

# Stub gh answers only the calls ci_truth_on_main_tip makes; scenario is env-driven.
mkdir -p "${TEST_ROOT}/bin2"
cat > "${TEST_ROOT}/bin2/gh" <<'EOF'
#!/usr/bin/env bash
case "$*" in
    *"--event push"*)       printf '%s|%s|%s|%s\n' "$FIX_HEAD" "$FIX_STATUS" "$FIX_CONCLUSION" "$FIX_RUNID" ;;
    *"run view"*databaseId*) printf '%s\n' "$FIX_JOBID" ;;
    *"run view"*)           printf '%s\n' "$FIX_JOBNAME" ;;
    *api*)                  printf '%s\n' "$FIX_STEP" ;;
    *)                     : ;;
esac
EOF
chmod 755 "${TEST_ROOT}/bin2/gh"

ci_truth_assert() {
    local label="$1" expect="$2" out
    export FIX_HEAD="$3" FIX_STATUS="$4" FIX_CONCLUSION="$5" FIX_RUNID="$6"
    export FIX_JOBID="$7" FIX_JOBNAME="$8" FIX_STEP="$9"
    out=$(PATH="${TEST_ROOT}/bin2:${PATH}" GOAP_REPO_ROOT="${REPO2}" \
        "${REPO2}/scripts/goap-orchestrator.sh" status 2>&1 \
        | sed $'s/\033\\[[0-9;]*m//g' | grep '^ci-truth' || true)
    if [[ "${out}" != *"${expect}"* ]]; then
        printf 'ci-truth %s: expected %s in: %s\n' "${label}" "${expect}" "${out}" >&2
        exit 1
    fi
    printf '  ok ci-truth %s\n' "${label}"
}

ci_truth_assert green        "state=green run=42000000001" "${TIP}" completed success   42000000001 0 "" ""
ci_truth_assert pending      "state=pending run=42000000002 status=in_progress" "${TIP}" in_progress "" 42000000002 0 "" ""
ci_truth_assert red-step     "state=red run=42000000003 conclusion=failure failing_step=job=lint step=LOC gate" "${TIP}" completed failure 42000000003 999 lint "LOC gate"
ci_truth_assert stale-head   "state=unverified" "deadbeef" completed success 42000000004 0 "" ""

# Offline: no gh on PATH at all must still print an unavailable line, never crash.
out=$(PATH="${TEST_ROOT}/bin:${PATH}" GOAP_REPO_ROOT="${REPO2}" \
    "${REPO2}/scripts/goap-orchestrator.sh" status 2>&1 \
    | sed $'s/\033\\[[0-9;]*m//g' | grep '^ci-truth' || true)
if [[ "${out}" != *"state=unavailable"* ]]; then
    printf 'ci-truth offline: expected state=unavailable in: %s\n' "${out}" >&2
    exit 1
fi
printf '  ok ci-truth offline\n'

printf 'GOAP ci-truth fixture: passed\n'
