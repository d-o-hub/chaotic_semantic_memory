#!/usr/bin/env bash
# GOAP Orchestrator - Manages GitHub issues with GOAP planning
# Usage: ./scripts/goap-orchestrator.sh <command> [args]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLANS_DIR="$REPO_ROOT/plans"
STATE_FILE="$PLANS_DIR/GOAP_ORCHESTRATOR.md"

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

    # Get all open issues
    local issues
    issues=$(gh issue list --state open --limit 50 --json number,title,labels \
        --jq '.[] | "\(.number)|\(.title)|\(.labels[].name // "none")"')

    # Create plan file
    cat > "$STATE_FILE" << 'EOF'
# GOAP Orchestrator State

## Target State
- All workspace extraction issues resolved
- CI passes for all crates
- Documentation reflects workspace structure

## Action Plan

EOF

    # Analyze and create actions
    local action_id=1
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
            } >> "$STATE_FILE"
            ((action_id++))
        fi
    done <<< "$issues"

    echo -e "${GREEN}Plan created: $STATE_FILE${NC}"
    cat "$STATE_FILE"
}


cmd_execute() {
    local action_num="${1:-1}"
    echo -e "${BLUE}=== Executing Action #$action_num ===${NC}"
    
    # Get issue number from plan
    local issue_num
    issue_num=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Issue" | head -1 | sed 's/.*#\([0-9]*\).*/\1/')
    
    if [[ -z "$issue_num" ]]; then
        echo -e "${RED}Action #$action_num not found in plan${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}Working on Issue #$issue_num${NC}"
    
    # Get issue details
    gh issue view "$issue_num" --json title,body,labels
    
    # Create branch
    local branch
    branch=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Branch" | head -1 | sed 's/.*: //')
    
    echo -e "${YELLOW}Creating branch: $branch${NC}"
    git checkout -b "$branch" 2>/dev/null || git checkout "$branch"
    
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

    # Current action_last_completed
    local last_completed
    last_completed=$(grep '^  action_last_completed:' "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null | sed 's/.*: //' || echo "unknown")
    echo -e "Last completed: ${GREEN}$last_completed${NC}"

    # Count queued actions
    local queued_count
    queued_count=$(grep -c 'status: queued' "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo "0")
    echo -e "Queued actions: ${YELLOW}$queued_count${NC}"

    # Count in-progress actions
    local in_progress_count
    in_progress_count=$(grep -c 'status: in_progress' "$PLANS_DIR/ACTIONS.md" 2>/dev/null || echo "0")
    echo -e "In-progress: ${YELLOW}$in_progress_count${NC}"

    # Active wave
    local active_wave
    active_wave=$(grep 'active_wave:' "$PLANS_DIR/GOAP_STATE.md" 2>/dev/null | sed 's/.*: //' || echo "unknown")
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

    if [[ ! -f "$STATE_FILE" ]]; then
        echo -e "${RED}No plan found. Run 'plan' first.${NC}"
        return 1
    fi

    sed -i "s/\(Action $action_num:.*\)/\1/; /Action $action_num:/,/^$/ s/status: queued/status: completed/" "$STATE_FILE"
    sed -i "/Action $action_num:/,/^$/ s/status: in_progress/status: completed/" "$STATE_FILE"

    echo -e "${GREEN}Action #$action_num marked complete${NC}"
}

cmd_help() {
    echo "GOAP Orchestrator - Manage GitHub issues with GOAP planning"
    echo ""
    echo "Usage: $0 <command> [args]"
    echo ""
    echo "Commands:"
    echo "  scan              List all open GitHub issues"
    echo "  plan              Generate GOAP action plan"
    echo "  status            Show current wave + in-progress actions + open PRs"
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
