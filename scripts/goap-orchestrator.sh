#!/usr/bin/env bash
# GOAP Orchestrator - Manages GitHub issues with GOAP planning
# Usage: ./scripts/goap-orchestrator.sh <command> [args]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${GOAP_REPO_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"
PLANS_DIR="$REPO_ROOT/plans"
STATE_FILE="$PLANS_DIR/GOAP_ORCHESTRATOR.md"
STATE_REL="plans/GOAP_ORCHESTRATOR.md"
cd "$REPO_ROOT"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ============================================================================
# Commands
# ============================================================================

cmd_scan() {
    echo -e "${BLUE}=== Scanning Open GitHub Issues ===${NC}"
    gh issue list --state open --limit 50 --json number,title,labels,createdAt \
        --jq '.[] | "\(.number)\t\(.title)\t\(.labels[].name // "none")\t\(.createdAt)"' \
        | sort -t$'\t' -k1 -n
    
    echo ""
    echo -e "${BLUE}=== Issue Count ===${NC}"
    gh issue list --state open --limit 50 --json number | jq length
}

cmd_plan() {
    echo -e "${BLUE}=== Building GOAP Plan ===${NC}"

    if ! git ls-files --error-unmatch "$STATE_REL" >/dev/null 2>&1; then
        echo -e "${RED}Generated plan must be tracked before it can be replaced${NC}" >&2
        return 1
    fi
    if ! git diff --quiet -- "$STATE_REL" || ! git diff --cached --quiet -- "$STATE_REL"; then
        echo -e "${RED}Generated plan has local changes; preserve or commit them before regenerating${NC}" >&2
        return 1
    fi

    local issues plan_tmp action_id number title labels
    issues=$(gh issue list --state open --limit 50 --json number,title,labels \
        --jq '.[] | "\(.number)|\(.title)|\(.labels[].name // "none")"') || return 1
    plan_tmp=$(mktemp "$PLANS_DIR/.GOAP_ORCHESTRATOR.XXXXXX")

    if ! {
        cat <<'EOF'
# GOAP Orchestrator State

## Target State
- All workspace extraction issues resolved
- CI passes for all crates
- Documentation reflects workspace structure

## Action Plan

EOF
        action_id=1
        while IFS='|' read -r number title labels; do
            if [[ "$number" =~ ^[0-9]+$ ]]; then
                {
                    echo "### Action $action_id: Issue #$number"
                    echo "- **Issue**: #$number"
                    echo "- **Title**: $title"
                    echo "- **Labels**: $labels"
                    echo "- **Status**: queued"
                    echo "- **Branch**: feat/issue-$number-$(echo "$title" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g' | head -c 50)"
                    echo ""
                }
                ((action_id++))
            fi
        done <<< "$issues"
    } > "$plan_tmp"; then
        rm -f -- "$plan_tmp"
        return 1
    fi

    chmod 644 "$plan_tmp"
    mv -f -- "$plan_tmp" "$STATE_FILE"
    echo -e "${GREEN}Plan created: $STATE_FILE${NC}"
    cat "$STATE_FILE"
}


cmd_execute() {
    local action_num="${1:-1}"
    echo -e "${BLUE}=== Executing Action #$action_num ===${NC}"

    if ! git diff --quiet || ! git diff --cached --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
        echo -e "${RED}Working tree must be clean before executing an action${NC}" >&2
        return 1
    fi

    if [[ ! "$action_num" =~ ^[1-9][0-9]*$ ]]; then
        echo -e "${RED}Action number must be a positive integer${NC}" >&2
        return 1
    fi

    local issue_num
    issue_num=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Issue" | head -1 | sed 's/.*#\([0-9]*\).*/\1/')
    if [[ -z "$issue_num" ]]; then
        echo -e "${RED}Action #$action_num not found in plan${NC}"
        return 1
    fi

    local action_status
    action_status=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Status" | head -1 | sed 's/.*: //')
    if [[ "$action_status" != "queued" && "$action_status" != "in_progress" ]]; then
        echo -e "${RED}Action #$action_num is not queued or in progress${NC}" >&2
        return 1
    fi

    echo -e "${YELLOW}Working on Issue #$issue_num${NC}"
    gh issue view "$issue_num" --json title,body,labels

    local branch current_branch
    branch=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Branch" | head -1 | sed 's/.*: //')
    current_branch=$(git branch --show-current)

    if [[ -z "$branch" ]]; then
        echo -e "${RED}Action #$action_num has no branch in the plan${NC}" >&2
        return 1
    fi
    if [[ -z "$current_branch" ]]; then
        echo -e "${RED}Detached HEAD is not allowed for action execution${NC}" >&2
        return 1
    fi
    if [[ "$current_branch" == "$branch" ]]; then
        echo -e "${GREEN}Already on action branch: $branch${NC}"
        return 0
    fi
    if git show-ref --verify --quiet "refs/heads/$branch"; then
        echo -e "${RED}Action branch already exists; inspect and switch to it explicitly${NC}" >&2
        return 1
    fi

    echo -e "${YELLOW}Creating branch: $branch${NC}"
    git switch --create "$branch"
    echo -e "${GREEN}Branch ready: $branch${NC}"
    echo -e "${YELLOW}Implement changes for Issue #$issue_num${NC}"
}

cmd_verify() {
    echo -e "${BLUE}=== Verifying CI Status ===${NC}"

    # Check latest workflow runs
    gh run list --workflow=ci.yml --limit 5 --json status,conclusion,headBranch \
        --jq '.[] | "\(.headBranch)\t\(.status)\t\(.conclusion // "pending")"'

    echo ""
    echo -e "${BLUE}=== Checking Branch Status ===${NC}"
    git status --short

    echo ""
    echo -e "${BLUE}=== LOC Gate ===${NC}"
    local over_limit
    over_limit=$(find "$REPO_ROOT/src" "$REPO_ROOT/crates" -name '*.rs' -not -path '*/target/*' \
        -exec wc -l {} + 2>/dev/null | sort -rn | awk '$1 > 500 && $2 != "total" {print "OVER: " $0}')
    if [[ -z "$over_limit" ]]; then
        echo -e "${GREEN}All files ≤500 LOC${NC}"
    else
        echo -e "${RED}$over_limit${NC}"
    fi

    echo ""
    echo -e "${BLUE}=== ADR Parity ===${NC}"
    if [[ -x "$REPO_ROOT/scripts/check-adr-parity.sh" ]]; then
        "$REPO_ROOT/scripts/check-adr-parity.sh" && echo -e "${GREEN}ADR registry aligned${NC}" || echo -e "${RED}ADR registry misaligned${NC}"
    else
        echo "ADR parity script not found"
    fi
}

cmd_status() {
    echo -e "${BLUE}=== GOAP Orchestrator Status ===${NC}"

    # Current action_last_completed.  Strip an inline comment without treating
    # colons in the comment as part of the value.
    local last_completed
    last_completed=$(awk '
        /^  action_last_completed:/ {
            value=$0
            sub(/^  action_last_completed:[[:space:]]*/, "", value)
            sub(/[[:space:]]+#.*$/, "", value)
            print value
            exit
        }
    ' "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null || true)
    [[ -n "$last_completed" ]] || last_completed="unknown"
    echo -e "Last completed: ${GREEN}$last_completed${NC}"

    # Count statuses only inside machine-readable action blocks.  Historical
    # prose in ACTIONS.md also contains the words `status: queued`; a raw grep
    # made an empty queue look active.
    local queued_count
    queued_count=$(awk '
        /^  - name:/ { in_action=1; next }
        in_action && /^  - / { in_action=0 }
        in_action && /^    status: queued[[:space:]]*$/ { count++ }
        END { print count + 0 }
    ' "$PLANS_DIR/ACTIONS.md" 2>/dev/null)
    echo -e "Queued actions: ${YELLOW}$queued_count${NC}"

    # Count in-progress actions
    local in_progress_count
    in_progress_count=$(awk '
        /^  - name:/ { in_action=1; next }
        in_action && /^  - / { in_action=0 }
        in_action && /^    status: in_progress[[:space:]]*$/ { count++ }
        END { print count + 0 }
    ' "$PLANS_DIR/ACTIONS.md" 2>/dev/null)
    echo -e "In-progress: ${YELLOW}$in_progress_count${NC}"

    # Active wave
    local active_wave
    active_wave=$(awk '
        /^  active_wave:/ {
            value=$0
            sub(/^  active_wave:[[:space:]]*/, "", value)
            sub(/[[:space:]]+#.*$/, "", value)
            print value
            exit
        }
    ' "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null || true)
    [[ -n "$active_wave" ]] || active_wave="unknown"
    echo -e "Active wave: ${BLUE}$active_wave${NC}"

    # Open PRs
    echo ""
    echo -e "${BLUE}=== Open PRs ===${NC}"
    gh pr list --state open --json number,title,mergeable \
        --jq '.[] | "#\(.number): \(.mergeable) — \(.title)"' 2>/dev/null || echo "No gh auth"

    # CI status
    echo ""
    echo -e "${BLUE}=== Latest CI ===${NC}"
    gh run list --workflow=ci.yml --limit 3 --json status,conclusion,displayTitle \
        --jq '.[] | "\(.status): \(.conclusion // "pending") — \(.displayTitle)"' 2>/dev/null || echo "No gh auth"
}

cmd_wave() {
    local wave_num="${1:-}"
    if [[ -z "$wave_num" ]]; then
        echo -e "${RED}Usage: $0 wave <N>${NC}"
        return 1
    fi

    echo -e "${BLUE}=== Wave $wave_num Plan ===${NC}"

    # Find wave section in ACTIONS.md or GOAP_STATE.md
    local wave_pattern="wave.*$wave_num|Wave $wave_num"
    local matches
    matches=$(grep -En -i "$wave_pattern" "$PLANS_DIR/ACTIONS.md" "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null || true)

    if [[ -z "$matches" ]]; then
        echo -e "${YELLOW}No wave $wave_num entries found in ACTIONS.md or GOAP_STATE.md${NC}"
        return 0
    fi

    echo "$matches"
    echo ""

    # Show queued actions with their wave tag
    echo -e "${BLUE}=== Queued Actions for Wave $wave_num ===${NC}"
    awk -v wave="wave-$wave_num" '
        /^  - name:/ {name=$3}
        /wave:/ && $0 ~ wave {print "  - " name}
    ' "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo "None found"
}

cmd_complete() {
    local action_num="${1:-1}"
    echo -e "${BLUE}=== Marking Action #$action_num Complete ===${NC}"

    if [[ ! "$action_num" =~ ^[1-9][0-9]*$ ]]; then
        echo -e "${RED}Action number must be a positive integer${NC}" >&2
        return 1
    fi
    if [[ ! -f "$STATE_FILE" ]]; then
        echo -e "${RED}No plan found. Run 'plan' first.${NC}"
        return 1
    fi
    if ! git ls-files --error-unmatch "$STATE_REL" >/dev/null 2>&1; then
        echo -e "${RED}Generated plan must be tracked before it can be updated${NC}" >&2
        return 1
    fi
    if ! git diff --quiet -- "$STATE_REL" || ! git diff --cached --quiet -- "$STATE_REL"; then
        echo -e "${RED}Generated plan has local changes; preserve or commit them before updating${NC}" >&2
        return 1
    fi

    local updated_file
    updated_file=$(mktemp "$PLANS_DIR/.GOAP_ORCHESTRATOR.XXXXXX")
    if ! awk -v action="$action_num" '
        /^### Action [0-9]+:/ {
            id=$0
            sub(/^### Action /, "", id)
            sub(/:.*/, "", id)
            in_action=(id == action)
            if (in_action) found=1
        }
        in_action && /^- \*\*Status\*\*: (queued|in_progress)$/ {
            sub(/(queued|in_progress)$/, "completed")
            changed=1
        }
        { print }
        END { if (!found || !changed) exit 1 }
    ' "$STATE_FILE" > "$updated_file"; then
        rm -f -- "$updated_file"
        echo -e "${RED}Action #$action_num not found or not active${NC}" >&2
        return 1
    fi

    chmod 644 "$updated_file"
    mv -f -- "$updated_file" "$STATE_FILE"
    echo -e "${GREEN}Action #$action_num marked complete${NC}"
}

cmd_help() {
    echo "GOAP Orchestrator - Manage GitHub issues with GOAP planning"
    echo ""
    echo "Usage: $0 <command> [args]"
    echo ""
    echo "Commands:"
    echo "  scan              List all open GitHub issues"
    echo "  plan              Generate/replace issue plan (clean generated file required)"
    echo "  status            Read-only wave/action/PR/CI status"
    echo "  wave <N>          Display wave plan with parallel breakdown"
    echo "  execute [action]  Execute an action (default: 1)"
    echo "  verify            Check CI + ADR parity + LOC gate"
    echo "  complete [action] Mark action complete (default: 1)"
    echo "  help              Show this help"
}

# ============================================================================
# Main
# ============================================================================

case "${1:-help}" in
    scan) cmd_scan ;;
    plan) cmd_plan ;;
    status) cmd_status ;;
    wave) cmd_wave "${2:-}" ;;
    execute) cmd_execute "${2:-1}" ;;
    verify) cmd_verify ;;
    complete) cmd_complete "${2:-1}" ;;
    help|*) cmd_help ;;
esac
