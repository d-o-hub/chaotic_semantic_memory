#!/usr/bin/env bash
# =============================================================================
# self-fix-loop.sh - Automated commit → push → monitor → fix → retry cycle
# =============================================================================
# Usage: scripts/self-fix-loop.sh [--dry-run] [--max-iterations N] [--timeout SECS]
#
# Commits only explicitly staged changes. Refuses main, detached HEAD, staged/
# unstaged mixtures, and untracked work. Every push requires an exact-target
# confirmation; non-interactive use requires SELF_FIX_LOOP_CONFIRM_PUSH=origin/BRANCH@SHA.
#
# Flags:
#   --dry-run              Show actions without executing
#   --max-iterations N     Max retry attempts (default: 5)
#   --timeout SECS         Max wait per CI run (default: 600)
#   --branch BRANCH        Must match the current branch
#   --help                 Show this help message
#
# Exit codes:
#   0 - CI passed
#   1 - Max iterations reached
#   2 - Unrecoverable error
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
  cat <<'EOF'
Usage: scripts/self-fix-loop.sh [flags]

Automated CI fix loop for one existing feature branch.

Flags:
  --dry-run              Show actions without executing
  --max-iterations N     Max retry attempts (default: 5)
  --timeout SECS         Max wait per CI run (default: 600)
  --branch BRANCH        Must match the current branch
  --help                 Show this help message

Before running, stage only the intended changes and keep the worktree clean.
Pushes require typing `origin/BRANCH@SHA` at the prompt. Non-interactive runs
must set `SELF_FIX_LOOP_CONFIRM_PUSH` to that exact value after committing.

Exit codes:
  0 - CI passed
  1 - Max iterations reached
  2 - Unrecoverable error or safety refusal

Examples:
  scripts/self-fix-loop.sh --dry-run
  scripts/self-fix-loop.sh --max-iterations 3
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
CURRENT_BRANCH="$(git branch --show-current)"
if [[ -z "$CURRENT_BRANCH" ]]; then
  echo -e "${RED}Error: detached HEAD is not allowed${NC}" >&2
  exit 2
fi
if [[ "$CURRENT_BRANCH" == "main" ]]; then
  echo -e "${RED}Error: refusing to mutate or push main${NC}" >&2
  exit 2
fi
if [[ -n "$BRANCH" && "$BRANCH" != "$CURRENT_BRANCH" ]]; then
  echo -e "${RED}Error: --branch must match checked-out branch $CURRENT_BRANCH${NC}" >&2
  exit 2
fi
BRANCH="$CURRENT_BRANCH"
if ! git diff --quiet; then
  echo -e "${RED}Error: unstaged changes present; stage only intended files${NC}" >&2
  exit 2
fi
if [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
  echo -e "${RED}Error: untracked files present; commit/preserve them separately${NC}" >&2
  exit 2
fi
if ! git remote get-url origin >/dev/null 2>&1; then
  echo -e "${RED}Error: origin remote is required${NC}" >&2
  exit 2
fi

if ! command -v gh >/dev/null 2>&1; then
  echo -e "${RED}Error: GitHub CLI (gh) required${NC}" >&2
  exit 2
fi
if ! gh auth status >/dev/null 2>&1; then
  echo -e "${RED}Error: not authenticated with GitHub; run gh auth login${NC}" >&2
  exit 2
fi

confirm_remote_push() {
  local target="origin/${BRANCH}@${PUSHED_SHA}"
  if [[ "${SELF_FIX_LOOP_CONFIRM_PUSH:-}" == "$target" ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    echo -e "${RED}Error: non-interactive push requires SELF_FIX_LOOP_CONFIRM_PUSH=$target${NC}" >&2
    return 1
  fi
  local confirmation=""
  read -r -p "Type ${target} to confirm this push: " confirmation
  [[ "$confirmation" == "$target" ]]
}

stage_automatic_fixes() {
  if [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    echo -e "${RED}Error: auto-fix created untracked files; inspect and stage explicitly${NC}" >&2
    return 1
  fi
  local -a changed_files=()
  mapfile -d '' -t changed_files < <(git diff --name-only -z)
  if [[ ${#changed_files[@]} -gt 0 ]]; then
    git add -- "${changed_files[@]}"
  fi
}

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

  # Step 1: Commit only changes the operator explicitly staged.
  echo -e "${BLUE}Step 1: Checking staged changes...${NC}"

  if git diff --staged --quiet; then
    echo -e "  ${GREEN}No staged changes to commit${NC}"
  elif $DRY_RUN; then
    echo -e "  ${YELLOW}[dry-run]${NC} Would commit staged changes"
  else
    git commit -m "fix: CI auto-fix iteration $ITERATION"
    echo -e "  ${GREEN}Staged changes committed${NC}"
  fi

  # Step 2: Push changes only after exact-target operator confirmation.
  echo -e "${BLUE}Step 2: Pushing to $BRANCH...${NC}"
  PUSHED_SHA="$(git rev-parse HEAD)"

  if $DRY_RUN; then
    echo -e "  ${YELLOW}[dry-run]${NC} Would push to origin/$BRANCH (SHA: $PUSHED_SHA)"
  else
    if ! confirm_remote_push; then
      echo -e "${RED}Push not confirmed; no remote mutation performed${NC}" >&2
      exit 2
    fi
    if ! git push origin "$BRANCH"; then
      echo -e "${RED}Push failed; no automatic fetch or rebase was attempted${NC}" >&2
      exit 2
    fi
    echo -e "  ${GREEN}Pushed to origin/$BRANCH (SHA: $PUSHED_SHA)${NC}"
  fi

  # Step 3: Monitor CI (bind to pushed commit SHA)
  echo -e "${BLUE}Step 3: Monitoring CI workflow...${NC}"

  if $DRY_RUN; then
    echo -e "  ${YELLOW}[dry-run]${NC} Would monitor CI for $TIMEOUT seconds"
    continue
  fi

  # Get the latest workflow run for this specific commit
  sleep 5  # Wait for CI to start

  # Filter by headSha to ensure we monitor the correct run
  RUN_ID="$(gh run list --branch "$BRANCH" --json databaseId,headSha --jq ".[] | select(.headSha == \"$PUSHED_SHA\") | .databaseId" | head -1)"

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
        cargo clippy --fix --allow-dirty --allow-staged
        ;;
      loc)
        echo -e "  ${YELLOW}LOC gate requires manual file splitting${NC}"
        ;;
      changelog)
        bash "$SCRIPT_DIR/validate-changelog.sh"
        ;;
      docs-sync)
        bash "$SCRIPT_DIR/sync-docs.sh"
        ;;
      *)
        echo -e "  ${YELLOW}No auto-fix available for: $FAILURE_TYPE${NC}"
        ;;
    esac

    echo -e "  Auto-fix applied. Will retry..."
    if ! stage_automatic_fixes; then
      exit 2
    fi
    if git diff --staged --quiet; then
      echo -e "${YELLOW}No automatic fix produced a staged change; stopping for manual action${NC}"
      exit 1
    fi
  fi
done

# Max iterations reached
echo -e "\n${RED}━━━ Max iterations ($MAX_ITERATIONS) reached ━━━${NC}"
echo -e "${YELLOW}CI still failing. Manual intervention required.${NC}"
echo "Last run view: gh run view $RUN_ID"
exit 1