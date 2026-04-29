#!/usr/bin/env bash
# =============================================================================
# ai-commit.sh - Helper for valid conventional commits with validation
# =============================================================================
# Usage: scripts/ai-commit.sh [options]
#
# Generates and validates conventional commit messages following:
#   https://www.conventionalcommits.org/en/v1.0.0/
#
# Options:
#   --auto          Auto-generate message from staged changes
#   --type TYPE     Commit type (feat|fix|docs|style|refactor|test|chore)
#   --scope SCOPE   Optional scope (e.g., cli, core, wasm)
#   --message MSG   Short description
#   --body BODY     Extended description (multi-line)
#   --breaking      Mark as breaking change
#   --dry-run       Show message without committing
#   --help          Show this help message
#
# Commit format:
#   <type>[optional scope]: <description>
#   [optional body]
#   [optional footer(s)]
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; NC='\033[0m'

# Valid commit types
VALID_TYPES="feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert"

# Flags
AUTO_MODE=false
COMMIT_TYPE=""
SCOPE=""
MESSAGE=""
BODY=""
BREAKING=false
DRY_RUN=false

# =============================================================================
# Help
# =============================================================================
show_help() {
  cat << EOF
Usage: scripts/ai-commit.sh [options]

Generate and validate conventional commit messages.

Options:
  --auto          Auto-generate message from staged changes
  --type TYPE     Commit type (feat|fix|docs|style|refactor|test|chore|perf|build|ci|revert)
  --scope SCOPE   Optional scope (e.g., cli, core, wasm)
  --message MSG   Short description (required unless --auto)
  --body BODY     Extended description (multi-line)
  --breaking      Mark as breaking change (adds ! after type)
  --dry-run       Show message without committing
  --help          Show this help message

Commit format:
  <type>[optional scope]: <description>
  [optional body]
  [optional footer(s)]

Examples:
  scripts/ai-commit.sh --auto                       # Auto-generate from staged changes
  scripts/ai-commit.sh --type feat --scope cli --message "add query command"
  scripts/ai-commit.sh --type fix --message "resolve clippy warnings" --breaking
  scripts/ai-commit.sh --type docs --message "update API docs" --dry-run

Type descriptions:
  feat     - New feature
  fix      - Bug fix
  docs     - Documentation changes
  style    - Code style (formatting, semicolons)
  refactor - Code refactoring (no feature/fix)
  perf     - Performance improvement
  test     - Adding/modifying tests
  build    - Build system changes
  ci       - CI configuration changes
  chore    - Maintenance tasks
  revert   - Revert previous commit
EOF
  exit 0
}

# =============================================================================
# Argument parsing
# =============================================================================
while [[ $# -gt 0 ]]; do
  case $1 in
    --auto)       AUTO_MODE=true; shift ;;
    --type)       COMMIT_TYPE="$2"; shift 2 ;;
    --scope)      SCOPE="$2"; shift 2 ;;
    --message)    MESSAGE="$2"; shift 2 ;;
    --body)       BODY="$2"; shift 2 ;;
    --breaking)   BREAKING=true; shift ;;
    --dry-run)    DRY_RUN=true; shift ;;
    --help|-h)    show_help ;;
    *)            echo "Unknown flag: $1"; exit 1 ;;
  esac
done

# =============================================================================
# Validation functions
# =============================================================================
validate_type() {
  local type="$1"
  if ! [[ "$type" =~ ^($VALID_TYPES)$ ]]; then
    echo -e "${RED}Error: Invalid commit type: $type${NC}"
    echo "Valid types: $VALID_TYPES"
    exit 1
  fi
}

validate_message() {
  local msg="$1"

  # Check length (recommended: 50 chars max, hard limit: 72)
  local len="${#msg}"
  if [[ $len -gt 72 ]]; then
    echo -e "${RED}Error: Message too long ($len chars, max 72)${NC}"
    echo "Message: $msg"
    exit 1
  fi

  if [[ $len -gt 50 ]]; then
    echo -e "${YELLOW}Warning: Message exceeds 50 chars ($len)${NC}"
  fi

  # Check for lowercase start (conventional)
  if [[ "$msg" =~ ^[A-Z] ]]; then
    echo -e "${YELLOW}Warning: Message starts with uppercase - conventional commits use lowercase${NC}"
  fi

  # Check for trailing period (not recommended)
  if [[ "$msg" =~ \.$ ]]; then
    echo -e "${YELLOW}Warning: Message ends with period - conventional commits omit trailing period${NC}"
  fi
}

# =============================================================================
# Auto-generate message from staged changes
# =============================================================================
auto_generate_message() {
  local diff_summary
  diff_summary="$(git diff --staged --stat)"

  if [[ -z "$diff_summary" ]]; then
    echo -e "${RED}Error: No staged changes to commit${NC}"
    echo "Stage changes first: git add <files>"
    exit 1
  fi

  # Analyze changes to infer type
  local files_changed
  files_changed="$(git diff --staged --name-only)"

  # Infer type from file patterns
  local inferred_type="chore"

  if echo "$files_changed" | grep -q "^src/.*\.rs$"; then
    if git diff --staged --unified=0 | grep -q "^[+-].*fn "; then
      inferred_type="feat"
    elif git diff --staged --unified=0 | grep -q "^[+-].*fix\|^[+-].*bug"; then
      inferred_type="fix"
    elif git diff --staged --unified=0 | grep -q "^[+-].*test\|^[+-].*#\[test\]"; then
      inferred_type="test"
    else
      inferred_type="refactor"
    fi
  elif echo "$files_changed" | grep -q "^docs/\|^.*\.md$"; then
    inferred_type="docs"
  elif echo "$files_changed" | grep -q "^\.github/workflows/"; then
    inferred_type="ci"
  elif echo "$files_changed" | grep -q "^scripts/"; then
    inferred_type="chore"
  fi

  COMMIT_TYPE="$inferred_type"

  # Infer scope from primary directory
  local primary_dir
  primary_dir="$(echo "$files_changed" | head -1 | cut -d/ -f1)"
  if [[ "$primary_dir" == "src" ]]; then
    local subdir
    subdir="$(echo "$files_changed" | head -1 | cut -d/ -f2)"
    if [[ "$subdir" == "bin" ]]; then
      SCOPE="cli"
    elif [[ -n "$subdir" && "$subdir" != "*.rs" ]]; then
      SCOPE="$subdir"
    fi
  elif [[ "$primary_dir" == ".github" ]]; then
    SCOPE="ci"
  elif [[ "$primary_dir" == "scripts" ]]; then
    SCOPE="scripts"
  fi

  # Generate description from diff stats
  local changed_count
  changed_count="$(echo "$files_changed" | wc -l | tr -d ' ')"

  if [[ $changed_count -eq 1 ]]; then
    local file_name
    file_name="$(echo "$files_changed" | head -1)"
    MESSAGE="update $(basename "$file_name")"
  else
    MESSAGE="update $changed_count files"
  fi

  echo -e "${CYAN}Auto-generated commit:${NC}"
  echo -e "  Type: $COMMIT_TYPE"
  echo -e "  Scope: ${SCOPE:-none}"
  echo -e "  Message: $MESSAGE"
}

# =============================================================================
# Build commit message
# =============================================================================
build_commit_message() {
  local header
  local breaking_marker=""

  if $BREAKING; then
    breaking_marker="!"
  fi

  if [[ -n "$SCOPE" ]]; then
    header="${COMMIT_TYPE}${breaking_marker}(${SCOPE}): ${MESSAGE}"
  else
    header="${COMMIT_TYPE}${breaking_marker}: ${MESSAGE}"
  fi

  if [[ -n "$BODY" ]]; then
    echo "$header"
    echo ""
    echo "$BODY"
  else
    echo "$header"
  fi
}

# =============================================================================
# Main flow
# =============================================================================
cd "$REPO_ROOT"

# Check git status
if ! git rev-parse --is-inside-work-tree &> /dev/null; then
  echo -e "${RED}Error: Not in a git repository${NC}"
  exit 1
fi

# Auto mode or manual mode
if $AUTO_MODE; then
  auto_generate_message
else
  # Manual mode: require type and message
  if [[ -z "$COMMIT_TYPE" ]]; then
    echo -e "${RED}Error: --type required (or use --auto)${NC}"
    exit 1
  fi

  if [[ -z "$MESSAGE" ]]; then
    echo -e "${RED}Error: --message required (or use --auto)${NC}"
    exit 1
  fi
fi

# Validate
validate_type "$COMMIT_TYPE"
validate_message "$MESSAGE"

# Build full message
FULL_MESSAGE="$(build_commit_message)"

echo -e "\n${CYAN}Generated commit message:${NC}"
echo -e "${GREEN}────────────────────────────────────${NC}"
echo "$FULL_MESSAGE"
echo -e "${GREEN}────────────────────────────────────${NC}"

# Dry run or commit
if $DRY_RUN; then
  echo -e "${YELLOW}Dry-run mode - no commit created${NC}"
  exit 0
fi

# Check for staged changes
if git diff --staged --quiet; then
  echo -e "${YELLOW}Warning: No staged changes. Staging all changes...${NC}"
  git add -A
fi

# Commit
git commit -m "$FULL_MESSAGE"

echo -e "${GREEN}Commit created successfully!${NC}"
echo "View: git log -1 --oneline"