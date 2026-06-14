#!/usr/bin/env bash
# GOAP Orchestrator - Manages GitHub issues with GOAP planning
# Usage: ./scripts/goap-orchestrator.sh <command> [args]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLANS_DIR="$REPO_ROOT/plans"
STATE_FILE="$PLANS_DIR/GOAP_ORCHESTRATOR.md"
TRACKER_FILE="$PLANS_DIR/ISSUE_TRACKER.md"

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
    local issues=$(gh issue list --state open --limit 50 --json number,title,labels \
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
            echo "### Action $action_id: Issue #$number" >> "$STATE_FILE"
            echo "- **Issue**: #$number" >> "$STATE_FILE"
            echo "- **Title**: $title" >> "$STATE_FILE"
            echo "- **Labels**: $labels" >> "$STATE_FILE"
            echo "- **Status**: queued" >> "$STATE_FILE"
            echo "- **Branch**: feat/issue-$number-$(echo "$title" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g' | head -c 50)" >> "$STATE_FILE"
            echo "" >> "$STATE_FILE"
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
    local issue_num=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Issue" | head -1 | sed 's/.*#\([0-9]*\).*/\1/')
    
    if [[ -z "$issue_num" ]]; then
        echo -e "${RED}Action #$action_num not found in plan${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}Working on Issue #$issue_num${NC}"
    
    # Get issue details
    gh issue view "$issue_num" --json title,body,labels
    
    # Create branch
    local branch=$(grep -A5 "Action $action_num:" "$STATE_FILE" | grep "Branch" | head -1 | sed 's/.*: //')
    
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
}

cmd_complete() {
    local action_num="${1:-1}"
    echo -e "${BLUE}=== Completing Action #$action_num ===${NC}"
    
    # Update plan state
    sed -i "s/### Action $action_num:.*$/### Action $action_num: [COMPLETED]/" "$STATE_FILE"
    
    # Commit and push
    local branch=$(git branch --show-current)
    echo -e "${YELLOW}Committing changes on $branch${NC}"
    
    git add -A
    git commit -m "feat(workspace): complete issue extraction" || echo "Nothing to commit"
    
    echo -e "${YELLOW}Pushing to remote${NC}"
    git push origin "$branch" 2>/dev/null || echo "Push failed or no remote"
    
    echo -e "${GREEN}Action #$action_num completed${NC}"
}

cmd_help() {
    echo "GOAP Orchestrator - Manage GitHub issues with GOAP planning"
    echo ""
    echo "Usage: $0 <command> [args]"
    echo ""
    echo "Commands:"
    echo "  scan              List all open GitHub issues"
    echo "  plan              Generate GOAP action plan"
    echo "  execute [action]  Execute an action (default: 1)"
    echo "  verify            Check CI status"
    echo "  complete [action] Mark action complete (default: 1)"
    echo "  help              Show this help"
}

# ============================================================================
# Main
# ============================================================================

case "${1:-help}" in
    scan) cmd_scan ;;
    plan) cmd_plan ;;
    execute) cmd_execute "${2:-1}" ;;
    verify) cmd_verify ;;
    complete) cmd_complete "${2:-1}" ;;
    help|*) cmd_help ;;
esac
