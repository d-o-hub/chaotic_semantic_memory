#!/usr/bin/env bash
# =============================================================================
# validate-git-hooks.sh - Prevent global hooks override
# =============================================================================
# Usage: scripts/validate-git-hooks.sh [--check] [--warn-only]
#
# Validates git hook configuration:
#   - Checks for local hooks in .git/hooks/
#   - Warns about global hooks that may override local
#   - Verifies hooks are executable
#
# Flags:
#   --check         Run validation checks (exit 1 if issues)
#   --warn-only     Print warnings but don't exit on errors
#   --install       Install local hooks from scripts/
#   --help          Show this help message
#
# Exit codes:
#   0 - All checks passed
#   1 - Issues found (unless --warn-only)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

# Flags
CHECK_MODE=false
WARN_ONLY=false
INSTALL_MODE=false

# =============================================================================
# Help
# =============================================================================
show_help() {
  cat << EOF
Usage: scripts/validate-git-hooks.sh [flags]

Validate git hook configuration to prevent global hooks override.

Flags:
  --check         Run validation checks (exit 1 if issues)
  --warn-only     Print warnings but don't exit on errors
  --install       Install local hooks from scripts/
  --help          Show this help message

Checks performed:
  1. Local hooks exist in .git/hooks/
  2. Hooks are executable (chmod 755)
  3. Global hooks don't override local
  4. Hooks are from project scripts (not external)

Examples:
  scripts/validate-git-hooks.sh --check      # Validate and exit 1 on issues
  scripts/validate-git-hooks.sh --warn-only  # Just print warnings
  scripts/validate-git-hooks.sh --install    # Install hooks from scripts/
EOF
  exit 0
}

# =============================================================================
# Argument parsing
# =============================================================================
while [[ $# -gt 0 ]]; do
  case $1 in
    --check)     CHECK_MODE=true; shift ;;
    --warn-only) WARN_ONLY=true; shift ;;
    --install)   INSTALL_MODE=true; shift ;;
    --help|-h)   show_help ;;
    *)           echo "Unknown flag: $1"; exit 1 ;;
  esac
done

# =============================================================================
# Install hooks
# =============================================================================
install_hooks() {
  echo -e "${CYAN}Installing git hooks...${NC}"

  if [[ ! -d "$HOOKS_DIR" ]]; then
    echo -e "${RED}Error: .git/hooks directory not found${NC}"
    echo "Are you in a git repository?"
    exit 1
  fi

  # Install pre-commit hook
  if [[ -f "$SCRIPT_DIR/pre-commit.sh" ]]; then
    cp "$SCRIPT_DIR/pre-commit.sh" "$HOOKS_DIR/pre-commit"
    chmod 755 "$HOOKS_DIR/pre-commit"
    echo -e "  ${GREEN}Installed:${NC} pre-commit"
  else
    echo -e "  ${YELLOW}Missing:${NC} scripts/pre-commit.sh"
  fi

  # Check for other common hooks in scripts/
  for hook in pre-push commit-msg; do
    if [[ -f "$SCRIPT_DIR/${hook}.sh" ]]; then
      cp "$SCRIPT_DIR/${hook}.sh" "$HOOKS_DIR/$hook"
      chmod 755 "$HOOKS_DIR/$hook"
      echo -e "  ${GREEN}Installed:${NC} $hook"
    fi
  done

  echo -e "\n${GREEN}Hooks installed successfully!${NC}"
}

# =============================================================================
# Validation checks
# =============================================================================
check_hooks() {
  echo -e "${CYAN}Validating git hooks configuration...${NC}\n"

  local issues=0

  # Check .git/hooks directory
  if [[ ! -d "$HOOKS_DIR" ]]; then
    echo -e "${RED}Error: .git/hooks directory not found${NC}"
    echo "Are you in a git repository?"
    exit 1
  fi

  # Check for local hooks
  echo -e "${BLUE}Checking local hooks...${NC}"

  local required_hooks="pre-commit"
  local found_hooks=0

  for hook in $required_hooks; do
    if [[ -f "$HOOKS_DIR/$hook" ]]; then
      # Check if executable
      if [[ -x "$HOOKS_DIR/$hook" ]]; then
        echo -e "  ${GREEN}OK:${NC} $hook (executable)"

        # Verify it's from our scripts (check for known patterns)
        if grep -q "chaotic_semantic_memory\|set -euo pipefail\|LOC gate" "$HOOKS_DIR/$hook" 2>/dev/null; then
          echo -e "    ${GREEN}Verified:${NC} Project hook detected"
        else
          echo -e "    ${YELLOW}Warning:${NC} Hook may not be from project scripts"
          ((issues++)) || true
        fi
        ((found_hooks++)) || true
      else
        echo -e "  ${RED}Error:${NC} $hook exists but not executable"
        echo "    Fix: chmod 755 $HOOKS_DIR/$hook"
        ((issues++)) || true
      fi
    else
      echo -e "  ${YELLOW}Missing:${NC} $hook"
      echo "    Install: scripts/validate-git-hooks.sh --install"
      ((issues++)) || true
    fi
  done

  # Check global hooks configuration
  echo -e "\n${BLUE}Checking global hooks configuration...${NC}"

  local global_hooks_path
  global_hooks_path="$(git config --global core.hooksPath 2>/dev/null || echo "")"

  if [[ -n "$global_hooks_path" ]]; then
    echo -e "  ${YELLOW}Warning:${NC} Global hooks path is set: $global_hooks_path"
    echo -e "    ${YELLOW}This may override local hooks!${NC}"
    echo "    Local hooks in .git/hooks/ will NOT run if global hooks are configured"
    echo "    Fix: git config --global --unset core.hooksPath"
    ((issues++)) || true
  else
    echo -e "  ${GREEN}OK:${NC} No global hooks override"
  fi

  # Check for hook templates (hooks installed by other tools)
  echo -e "\n${BLUE}Checking for external hooks...${NC}"

  for hook in pre-commit post-commit commit-msg; do
    if [[ -f "$HOOKS_DIR/$hook.sample" ]]; then
      echo -e "  ${YELLOW}Note:${NC} $hook.sample exists (git template)"
    fi
  done

  # Summary
  echo -e "\n${CYAN}━━━ Summary ━━━${NC}"

  if [[ $issues -eq 0 ]]; then
    echo -e "${GREEN}All hook checks passed!${NC}"
    return 0
  else
    echo -e "${YELLOW}Issues found: $issues${NC}"

    if $WARN_ONLY; then
      echo -e "${YELLOW}Warnings printed (--warn-only mode)${NC}"
      return 0
    else
      echo -e "${RED}Validation failed${NC}"
      return 1
    fi
  fi
}

# =============================================================================
# Main flow
# =============================================================================
cd "$REPO_ROOT"

# Verify git repo
if ! git rev-parse --is-inside-work-tree &> /dev/null; then
  echo -e "${RED}Error: Not in a git repository${NC}"
  exit 1
fi

if $INSTALL_MODE; then
  install_hooks
elif $CHECK_MODE || [[ $# -eq 0 ]]; then
  check_hooks
  exit_code=$?
  if ! $WARN_ONLY && [[ $exit_code -ne 0 ]]; then
    exit $exit_code
  fi
else
  # Default: run checks in warn-only mode
  WARN_ONLY=true
  check_hooks
fi