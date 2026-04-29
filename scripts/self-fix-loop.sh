#!/usr/bin/env bash
# =============================================================================
# self-fix-loop.sh - Automated commit → push → monitor → fix → retry cycle
# =============================================================================
# Usage: scripts/self-fix-loop.sh [--dry-run] [--max-iterations N] [--timeout SECS]
#
# Automates the CI fix loop:
#   1. Commit current changes
#   2. Push to remote
#   3. Monitor CI workflow
#   4. If failure, classify and suggest fix
#   5. Retry until success or max iterations
#
# Flags:
#   --dry-run              Show actions without executing
#   --max-iterations N     Max retry attempts (default: 5)
#   --timeout SECS         Max wait per CI run (default: 600)
#   --branch BRANCH        Target branch (default: current)
#   --help                 Show this help message
#
# Exit codes:
#   0 - CI passed
#   1 - Max iterations reached
#   2 - Unrecoverable error
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

# Defaults
DRY_RUN=false
MAX_ITERATIONS=5
TIMEOUT=600
BRANCH=""

# =============================================================================
# Help
# =============================================================================
show_help() {
  cat << EOF
Usage: scripts/self-fix-loop.sh [flags]

Automated CI fix loop: commit → push → monitor → fix → retry.

Flags:
  --dry-run              Show actions without executing
  --max-iterations N     Max retry attempts (default: 5)
  --timeout SECS         Max wait per CI run (default: 600)
  --branch BRANCH        Target branch (default: current)
  --help                 Show this help message

Exit codes:
  0 - CI passed
  1 - Max iterations reached
  2 - Unrecoverable error

Examples:
  scripts/self-fix-loop.sh                           # Run with defaults
  scripts/self-fix-loop.sh --max-iterations 3        # Limit to 3 attempts
  scripts/self-fix-loop.sh --dry-run                 # Preview actions
EOF
  exit 0
}

# =============================================================================
# Argument parsing
# =============================================================================
while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run)            DRY_RUN=true; shift ;;
    --max-iterations)     MAX_ITERATIONS="$2"; shift 2 ;;
    --timeout)            TIMEOUT="$2"; shift 2 ;;
    --branch)             BRANCH="$2"; shift 2 ;;
    --help|-h)            show_help ;;
    *)                    echo "Unknown flag: $1"; exit 1 ;;
  esac
done

# Validate dependencies
if ! command -v gh &> /dev/null; then
  echo -e "${RED}Error: GitHub CLI (gh) required. Install: https://cli.github.com/${NC}"
  exit 2
fi

if ! gh auth status &> /dev/null 2>&1; then
  echo -e "${RED}Error: Not authenticated with GitHub. Run: gh auth login${NC}"
  exit 2
fi

if ! command -v jq &> /dev/null; then
  echo -e "${RED}Error: jq required. Install: https://stedolan.github.io/jq/download/${NC}"
  exit 2
fi

# Get current branch
if [[ -z "$BRANCH" ]]; then
  BRANCH="$(git rev-parse --abbrev-ref HEAD)"
fi

echo -e "${CYAN}╔══════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║        Self-Fix Loop (CI Monitor)            ║${NC}"
echo -e "${CYAN}║  Branch: $BRANCH                              ║${NC}"
echo -e "${CYAN}║  Max iterations: $MAX_ITERATIONS              ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════╝${NC}"

if $DRY_RUN; then
  echo -e "${YELLOW}Running in DRY-RUN mode${NC}"
fi

# =============================================================================
# Failure classification
# =============================================================================
classify_failure() {
  local log="$1"
  local failure_type="unknown"
  local fix_hint=""

  # Check for common failure patterns
  if grep -qi "cargo fmt.*--check" "$log" || grep -qi "formatting" "$log"; then
    failure_type="formatting"
    fix_hint="Run: cargo fmt"
  elif grep -qi "cargo clippy" "$log" || grep -qi "warning:" "$log"; then
    failure_type="clippy"
    fix_hint="Run: cargo clippy --fix --allow-dirty"
  elif grep -qi "cargo test" "$log" || grep -qi "test.*FAILED" "$log"; then
    failure_type="test"
    fix_hint="Run: cargo test --all-features to identify failing tests"
  elif grep -qi "LOC gate" "$log" || grep -qi "lines" "$log"; then
    failure_type="loc"
    fix_hint="Split large file into extensions (max 500 LOC)"
  elif grep -qi "wasm_size_gate" "$log" || grep -qi "WASM size" "$log"; then
    failure_type="wasm-size"
    fix_hint="Reduce WASM binary size (strip, optimize)"
  elif grep -qi "CHANGELOG" "$log"; then
    failure_type="changelog"
    fix_hint="Fix CHANGELOG format (missing version header or duplicate)"
  elif grep -qi "docs version sync" "$log"; then
    failure_type="docs-sync"
    fix_hint="Run: scripts/sync-docs.sh"
  elif grep -qi "merge conflict" "$log" || grep -qi "CONFLICT" "$log"; then
    failure_type="merge-conflict"
    fix_hint="Resolve merge conflicts manually"
  elif grep -qi "timeout" "$log" || grep -qi "timed out" "$log"; then
    failure_type="timeout"
    fix_hint="CI timed out - check for long-running steps"
  fi

  echo "$failure_type|$fix_hint"
}

# =============================================================================
# Main loop
# =============================================================================
ITERATION=0

while [[ $ITERATION -lt $MAX_ITERATIONS ]]; do
  ((ITERATION++)) || true

  echo -e "\n${CYAN}━━━ Iteration $ITERATION/$MAX_ITERATIONS ━━━${NC}"

  # Step 1: Check for changes to commit
  echo -e "${BLUE}Step 1: Checking for changes...${NC}"

  if git diff --quiet && git diff --staged --quiet; then
    echo -e "  ${GREEN}No local changes to commit${NC}"
  else
    if $DRY_RUN; then
      echo -e "  ${YELLOW}[dry-run]${NC} Would commit staged changes"
    else
      # Commit with default message or use ai-commit.sh if available
      if [[ -f "$SCRIPT_DIR/ai-commit.sh" ]]; then
        echo -e "  Using ai-commit.sh for commit message..."
        git add -A
        bash "$SCRIPT_DIR/ai-commit.sh" --auto || git commit -m "fix: CI auto-fix iteration $ITERATION"
      else
        git add -A
        git commit -m "fix: CI auto-fix iteration $ITERATION"
      fi
      echo -e "  ${GREEN}Changes committed${NC}"
    fi
  fi

  # Step 2: Push changes
  echo -e "${BLUE}Step 2: Pushing to $BRANCH...${NC}"
  if $DRY_RUN; then
    echo -e "  ${YELLOW}[dry-run]${NC} Would push to origin/$BRANCH"
  else
    git push origin "$BRANCH" 2>&1 || {
      echo -e "  ${RED}Push failed${NC}"
      # Check for upstream changes
      git fetch origin
      if ! git diff --quiet "HEAD..origin/$BRANCH"; then
        echo -e "  ${YELLOW}Remote has new commits - rebasing...${NC}"
        git pull --rebase origin "$BRANCH"
        git push origin "$BRANCH"
      fi
    }
    echo -e "  ${GREEN}Pushed to origin/$BRANCH${NC}"
  fi

  # Step 3: Monitor CI
  echo -e "${BLUE}Step 3: Monitoring CI workflow...${NC}"

  if $DRY_RUN; then
    echo -e "  ${YELLOW}[dry-run]${NC} Would monitor CI for $TIMEOUT seconds"
    continue
  fi

  # Get the latest workflow run
  sleep 5  # Wait for CI to start

  HEAD_SHA="$(git rev-parse HEAD)"
  RUN_ID="$(gh run list --branch "$BRANCH" --commit "$HEAD_SHA" --limit 1 --json databaseId --jq '.[0].databaseId')"

  if [[ -z "$RUN_ID" || "$RUN_ID" == "null" ]]; then
    echo -e "  ${YELLOW}No workflow run found - waiting...${NC}"
    sleep 10
    continue
  fi

  echo -e "  Monitoring run: $RUN_ID"
  echo -e "  View: gh run view $RUN_ID"

  # Wait for completion with timeout
  START_TIME="$(date +%s)"

  while true; do
    STATUS="$(gh run view "$RUN_ID" --json status,conclusion --jq '{status: .status, conclusion: .conclusion}')"
    RUN_STATUS="$(echo "$STATUS" | jq -r '.status')"
    CONCLUSION="$(echo "$STATUS" | jq -r '.conclusion')"

    if [[ "$RUN_STATUS" == "completed" ]]; then
      break
    fi

    CURRENT_TIME="$(date +%s)"
    ELAPSED=$((CURRENT_TIME - START_TIME))

    if [[ $ELAPSED -gt $TIMEOUT ]]; then
      echo -e "  ${RED}Timeout after $TIMEOUT seconds${NC}"
      exit 2
    fi

    echo -e "  ${YELLOW}Waiting...${NC} ($ELAPSED/$TIMEOUT seconds, status: $RUN_STATUS)"
    sleep 30
  done

  # Step 4: Check result
  echo -e "${BLUE}Step 4: Checking CI result...${NC}"

  if [[ "$CONCLUSION" == "success" ]]; then
    echo -e "  ${GREEN}CI PASSED!${NC}"
    echo -e "\n${GREEN}━━━ Self-fix loop completed successfully! ━━━${NC}"
    echo "Iterations: $ITERATION"
    exit 0
  fi

  echo -e "  ${RED}CI FAILED with conclusion: $CONCLUSION${NC}"

  # Get failure logs
  echo -e "${BLUE}Step 5: Analyzing failure...${NC}"

  FAILED_JOBS="$(gh run view "$RUN_ID" --json jobs --jq '.jobs[] | select(.conclusion == "failure") | .name')"
  echo -e "  Failed jobs:"
  echo "$FAILED_JOBS" | while read -r job; do
    echo -e "    ${RED}- $job${NC}"
  done

  # Get logs for analysis
  LOG_FILE="$(mktemp)"
  gh run view "$RUN_ID" --log > "$LOG_FILE" 2>&1 || true

  # Classify failure
  CLASSIFICATION="$(classify_failure "$LOG_FILE")"
  FAILURE_TYPE="$(echo "$CLASSIFICATION" | cut -d'|' -f1)"
  FIX_HINT="$(echo "$CLASSIFICATION" | cut -d'|' -f2)"

  echo -e "\n  ${CYAN}Failure classification:${NC} $FAILURE_TYPE"
  echo -e "  ${CYAN}Fix suggestion:${NC} $FIX_HINT"

  rm -f "$LOG_FILE"

  # Check for unrecoverable errors
  if [[ "$FAILURE_TYPE" == "merge-conflict" ]]; then
    echo -e "\n${RED}━━━ Unrecoverable error: merge conflict ━━━${NC}"
    echo "Resolve conflicts manually and retry"
    exit 2
  fi

  # Auto-fix attempt (if not dry-run)
  if ! $DRY_RUN; then
    echo -e "\n${YELLOW}Attempting auto-fix...${NC}"

    case "$FAILURE_TYPE" in
      formatting)
        cargo fmt
        ;;
      clippy)
        cargo clippy --fix --allow-dirty --allow-staged 2>&1 || true
        ;;
      loc)
        echo -e "  ${YELLOW}LOC gate requires manual file splitting${NC}"
        ;;
      changelog)
        bash "$SCRIPT_DIR/validate-changelog.sh" 2>&1 || true
        ;;
      docs-sync)
        bash "$SCRIPT_DIR/sync-docs.sh" 2>&1 || true
        ;;
      *)
        echo -e "  ${YELLOW}No auto-fix available for: $FAILURE_TYPE${NC}"
        ;;
    esac

    echo -e "  Auto-fix applied. Will retry..."
  fi
done

# Max iterations reached
echo -e "\n${RED}━━━ Max iterations ($MAX_ITERATIONS) reached ━━━${NC}"
echo -e "${YELLOW}CI still failing. Manual intervention required.${NC}"
echo "Last run view: gh run view $RUN_ID"
exit 1