#!/usr/bin/env bash
# =============================================================================
# validate-git-hooks.sh - Validate git hook configuration & fail closed
# =============================================================================
# Usage: scripts/validate-git-hooks.sh [--check] [--warn-only] [--install]
#
# Validates git hook configuration:
#   - Checks local core.hooksPath configuration and .githooks directory
#   - Verifies required hooks (pre-commit, pre-push) exist and are executable
#   - Detects global hooks override or invalid local overrides
#   - Fails closed with exit status 1 if any hook is missing or unexecutable
#
# Flags:
#   --check         Run validation checks (exit 1 if issues)
#   --warn-only     Print warnings but don't exit on errors
#   --install       Install local hooks via scripts/install-hooks.sh
#   --help          Show this help message
#
# Exit codes:
#   0 - All checks passed
#   1 - Validation issues found (unless --warn-only)
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

# Flags
WARN_ONLY=false
INSTALL_MODE=false

show_help() {
  cat << EOF
Usage: scripts/validate-git-hooks.sh [flags]

Validate git hook configuration to prevent global hooks override and uninstalled hooks.

Flags:
  --check         Run validation checks (exit 1 if issues)
  --warn-only     Print warnings but don't exit on errors
  --install       Install local hooks via scripts/install-hooks.sh
  --help          Show this help message

Checks performed:
  1. core.hooksPath is set (repo-local or absolute .githooks)
  2. Required hooks (pre-commit, pre-push) exist in .githooks/ and are executable
  3. Global hooks don't override local
EOF
  exit 0
}

while [[ $# -gt 0 ]]; do
  case $1 in
    --check)     shift ;;
    --warn-only) WARN_ONLY=true; shift ;;
    --install)   INSTALL_MODE=true; shift ;;
    --help|-h)   show_help ;;
    *)           echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

install_hooks() {
  "$SCRIPT_DIR/install-hooks.sh"
}

check_hooks() {
  echo -e "${CYAN}Validating git hooks configuration...${NC}\n"

  local issues=0
  local required_hooks=("pre-commit" "pre-push")

  # 1. Check local core.hooksPath configuration
  echo -e "${BLUE}Checking local git config core.hooksPath...${NC}"
  local local_hooks_path
  local_hooks_path="$(git config --local --path core.hooksPath 2>/dev/null || true)"

  if [[ -z "$local_hooks_path" ]]; then
    echo -e "  ${RED}Error:${NC} Local core.hooksPath is not set." >&2
    echo "    Fix: run scripts/install-hooks.sh"
    issues=$((issues + 1))
  else
    local effective_hooks_path resolved_hooks_path expected_hooks_path resolved_local_path
    effective_hooks_path="$(git config --path --get core.hooksPath 2>/dev/null || true)"
    resolved_hooks_path="$(cd "$effective_hooks_path" 2>/dev/null && pwd -P || true)"
    expected_hooks_path="$(cd "$REPO_ROOT/.githooks" 2>/dev/null && pwd -P || true)"
    resolved_local_path="$(cd "$local_hooks_path" 2>/dev/null && pwd -P || true)"
    if [[ -z "$resolved_local_path" || "$resolved_local_path" != "$expected_hooks_path" ]]; then
      echo -e "  ${RED}Error:${NC} Local core.hooksPath does not select this checkout's .githooks: $local_hooks_path" >&2
      issues=$((issues + 1))
    elif [[ -z "$resolved_hooks_path" || "$resolved_hooks_path" != "$expected_hooks_path" ]]; then
      echo -e "  ${RED}Error:${NC} Effective core.hooksPath does not select this checkout's .githooks: $effective_hooks_path" >&2
      issues=$((issues + 1))
    else
      echo -e "  ${GREEN}OK:${NC} core.hooksPath selects this checkout's .githooks"
    fi
  fi

  # 2. Check global hooks configuration
  echo -e "\n${BLUE}Checking global hooks configuration...${NC}"
  local global_hooks_path
  global_hooks_path="$(git config --global core.hooksPath 2>/dev/null || echo "")"

  if [[ -n "$global_hooks_path" ]]; then
    echo -e "  ${YELLOW}Warning:${NC} Global core.hooksPath is set: $global_hooks_path"
    echo -e "    ${YELLOW}Ensure repo-local config overrides global!${NC}"
  else
    echo -e "  ${GREEN}OK:${NC} No global hooks override"
  fi

  # 3. Check .githooks directory and required hook scripts
  echo -e "\n${BLUE}Checking hook files in .githooks/...${NC}"
  local githooks_dir="$REPO_ROOT/.githooks"

  if [[ ! -d "$githooks_dir" ]]; then
    echo -e "  ${RED}Error:${NC} .githooks directory not found at $githooks_dir" >&2
    issues=$((issues + 1))
  else
    for hook in "${required_hooks[@]}"; do
      local hook_path="$githooks_dir/$hook"
      if [[ -f "$hook_path" ]]; then
        if [[ -x "$hook_path" ]]; then
          echo -e "  ${GREEN}OK:${NC} $hook exists and is executable"
        else
          echo -e "  ${RED}Error:${NC} $hook exists but is NOT executable" >&2
          echo "    Fix: chmod 755 $hook_path"
          issues=$((issues + 1))
        fi
      else
        echo -e "  ${RED}Error:${NC} Required hook missing: $hook ($hook_path)" >&2
        echo "    Fix: run scripts/install-hooks.sh"
        issues=$((issues + 1))
      fi
    done
  fi

  # Summary
  echo -e "\n${CYAN}━━━ Summary ━━━${NC}"

  if [[ $issues -eq 0 ]]; then
    echo -e "${GREEN}All hook checks passed!${NC}"
    return 0
  else
    echo -e "${RED}Issues found: $issues${NC}"

    if $WARN_ONLY; then
      echo -e "${YELLOW}Warnings printed (--warn-only mode)${NC}"
      return 0
    else
      echo -e "${RED}Validation failed.${NC}" >&2
      return 1
    fi
  fi
}

cd "$REPO_ROOT"

if ! git rev-parse --is-inside-work-tree &> /dev/null; then
  echo -e "${RED}Error: Not in a git repository${NC}" >&2
  exit 1
fi

if $INSTALL_MODE; then
  install_hooks
fi
check_hooks
