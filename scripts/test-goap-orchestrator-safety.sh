#!/usr/bin/env bash
# Isolated behavior checks for GOAP plan writes, completion, and branch safety.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT
REPO="$TEST_ROOT/repo"
mkdir -p "$REPO/plans" "$REPO/scripts" "$TEST_ROOT/bin"
cp "$SCRIPT_DIR/goap-orchestrator.sh" "$REPO/scripts/goap-orchestrator.sh"
chmod 755 "$REPO/scripts/goap-orchestrator.sh"

git init -q -b main "$REPO"
git -C "$REPO" config user.email fixture@example.invalid
git -C "$REPO" config user.name fixture
cat > "$REPO/plans/GOAP_ORCHESTRATOR.md" <<'EOF'
# GOAP Orchestrator State

## Target State

## Action Plan
EOF
git -C "$REPO" add plans/GOAP_ORCHESTRATOR.md scripts/goap-orchestrator.sh
git -C "$REPO" commit -qm 'fixture: initialize GOAP state'

cat > "$TEST_ROOT/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-} ${2:-}" in
    "issue list") printf '42|Atomic plan|P1\n' ;;
    "issue view") printf '{"title":"Atomic plan"}\n' ;;
    *) exit 0 ;;
esac
EOF
chmod 755 "$TEST_ROOT/bin/gh"
run_goap() {
    GOAP_REPO_ROOT="$REPO" PATH="$TEST_ROOT/bin:$PATH" \
        "$REPO/scripts/goap-orchestrator.sh" "$@"
}

run_goap plan > "$TEST_ROOT/plan.out"
grep -Fq '### Action 1: Issue #42' "$REPO/plans/GOAP_ORCHESTRATOR.md"
grep -Fq -- '- **Status**: queued' "$REPO/plans/GOAP_ORCHESTRATOR.md"
git -C "$REPO" add plans/GOAP_ORCHESTRATOR.md
git -C "$REPO" commit -qm 'fixture: generated plan'

printf '\nlocal edit\n' >> "$REPO/plans/GOAP_ORCHESTRATOR.md"
before="$(sha256sum "$REPO/plans/GOAP_ORCHESTRATOR.md")"
if run_goap plan > "$TEST_ROOT/dirty-plan.out" 2>&1; then
    echo 'plan unexpectedly replaced a locally edited generated file' >&2
    exit 1
fi
after="$(sha256sum "$REPO/plans/GOAP_ORCHESTRATOR.md")"
[[ "$before" == "$after" ]]
git -C "$REPO" restore -- plans/GOAP_ORCHESTRATOR.md

run_goap execute 1 > "$TEST_ROOT/execute.out"
[[ "$(git -C "$REPO" branch --show-current)" == 'feat/issue-42-atomic-plan' ]]
git -C "$REPO" switch -q main
if run_goap execute 1 > "$TEST_ROOT/existing-branch.out" 2>&1; then
    echo 'execute unexpectedly switched to an existing action branch' >&2
    exit 1
fi
[[ "$(git -C "$REPO" branch --show-current)" == main ]]

git -C "$REPO" status --porcelain | grep -q . && {
    echo 'working tree unexpectedly dirty before completing action' >&2
    exit 1
}
run_goap complete 1 > "$TEST_ROOT/complete.out"
grep -Fq -- '- **Status**: completed' "$REPO/plans/GOAP_ORCHESTRATOR.md"
git -C "$REPO" add plans/GOAP_ORCHESTRATOR.md
git -C "$REPO" commit -qm 'fixture: completed action'
if run_goap execute 1 > "$TEST_ROOT/completed-action.out" 2>&1; then
    echo 'execute unexpectedly accepted a completed action' >&2
    exit 1
fi
[[ "$(git -C "$REPO" branch --show-current)" == main ]]
if compgen -G "$REPO/plans/.GOAP_ORCHESTRATOR.*" >/dev/null; then
    echo 'GOAP command left a temporary plan file behind' >&2
    exit 1
fi

printf 'GOAP mutation safety fixtures: passed\n'
