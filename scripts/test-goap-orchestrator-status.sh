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
