#!/usr/bin/env bash
# =============================================================================
# validate-workflows.sh - YAML validation for GitHub Actions
# =============================================================================
# Usage: scripts/validate-workflows.sh [--check] [--fix]
#
# Validates GitHub Actions workflow files:
#   - YAML syntax validation
#   - Common workflow issues detection
#   - Security checks (action pinning, permissions)
#
# Flags:
#   --check         Validate all workflows (exit 1 on errors)
#   --fix           Attempt to fix common issues
#   --verbose       Show detailed validation output
#   --help          Show this help message
#
# Checks performed:
#   1. YAML syntax validity
#   2. Required fields (name, on, jobs)
#   3. Action SHA pinning (security)
#   4. Permissions declarations
#   5. Common syntax errors
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKFLOWS_DIR="$REPO_ROOT/.github/workflows"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

# Flags
CHECK_MODE=false
FIX_MODE=false
VERBOSE=false

# =============================================================================
# Help
# =============================================================================
show_help() {
  cat << EOF
Usage: scripts/validate-workflows.sh [flags]

Validate GitHub Actions workflow YAML files.

Flags:
  --check         Validate all workflows (exit 1 on errors)
  --fix           Attempt to fix common issues
  --verbose       Show detailed validation output
  --help          Show this help message

Checks performed:
  1. YAML syntax validity
  2. Required fields (name, on, jobs)
  3. Action SHA pinning (security best practice)
  4. Permissions declarations
  5. Common syntax errors

Examples:
  scripts/validate-workflows.sh              # Run basic validation
  scripts/validate-workflows.sh --check      # Strict validation (exit 1 on errors)
  scripts/validate-workflows.sh --verbose    # Detailed output
  scripts/validate-workflows.sh --fix        # Attempt fixes
EOF
  exit 0
}

# =============================================================================
# Argument parsing
# =============================================================================
while [[ $# -gt 0 ]]; do
  case $1 in
    --check)    CHECK_MODE=true; shift ;;
    --fix)      FIX_MODE=true; shift ;;
    --verbose)  VERBOSE=true; shift ;;
    --help|-h)  show_help ;;
    *)          echo "Unknown flag: $1"; exit 1 ;;
  esac
done

# =============================================================================
# Validation functions
# =============================================================================
validate_yaml_syntax() {
  local file="$1"
  local errors=0

  # Check for Python/yamllint
  if command -v yamllint &> /dev/null; then
    if ! yamllint -d relaxed "$file" 2>&1; then
      echo -e "  ${RED}YAML syntax error${NC}"
      ((errors++)) || true
    fi
  elif command -v python3 &> /dev/null; then
    # Use Python YAML parser
    if python3 -c "import yaml" 2>/dev/null; then
      if ! python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>&1; then
        echo -e "  ${RED}YAML syntax error${NC}"
        ((errors++)) || true
      fi
    else
      # PyYAML not installed, fallback to basic checks
      echo -e "  ${YELLOW}Note: yamllint/PyYAML not found - basic checks only${NC}"

      # Check for common YAML errors
      # Indentation consistency
      local indent_errors
      indent_errors="$(grep -n '^  [^ ]' "$file" | grep -v '^  [a-z]' || true)"
      if [[ -n "$indent_errors" ]]; then
        echo -e "  ${YELLOW}Possible indentation issue:${NC}"
        echo "$indent_errors" | head -3
        ((errors++)) || true
      fi

      # Check for trailing whitespace
      if grep -q ' $' "$file"; then
        echo -e "  ${YELLOW}Trailing whitespace detected${NC}"
      fi

      # Check for tabs (YAML should use spaces)
      if grep -q "$(printf '\t')" "$file"; then
        echo -e "  ${RED}Tabs detected - YAML requires spaces${NC}"
        ((errors++)) || true
      fi
    fi
  else
    # Basic syntax checks without external tools
    echo -e "  ${YELLOW}Note: yamllint/python3 not found - basic checks only${NC}"

    # Check for common YAML errors
    # Indentation consistency
    local indent_errors
    indent_errors="$(grep -n '^  [^ ]' "$file" | grep -v '^  [a-z]' || true)"
    if [[ -n "$indent_errors" ]]; then
      echo -e "  ${YELLOW}Possible indentation issue:${NC}"
      echo "$indent_errors" | head -3
      ((errors++)) || true
    fi

    # Check for trailing whitespace
    if grep -q ' $' "$file"; then
      echo -e "  ${YELLOW}Trailing whitespace detected${NC}"
    fi

    # Check for tabs (YAML should use spaces)
    if grep -q "$(printf '\t')" "$file"; then
      echo -e "  ${RED}Tabs detected - YAML requires spaces${NC}"
      ((errors++)) || true
    fi
  fi

  return $errors
}

check_required_fields() {
  local file="$1"
  local errors=0

  # Check for 'name:' field
  if ! grep -q "^name:" "$file"; then
    echo -e "  ${YELLOW}Missing 'name' field${NC}"
    ((errors++)) || true
  fi

  # Check for 'on:' trigger
  if ! grep -q "^on:" "$file"; then
    echo -e "  ${RED}Missing 'on' trigger definition${NC}"
    ((errors++)) || true
  fi

  # Check for 'jobs:' section
  if ! grep -q "^jobs:" "$file"; then
    echo -e "  ${RED}Missing 'jobs' section${NC}"
    ((errors++)) || true
  fi

  return $errors
}

check_action_pinning() {
  local file="$1"
  local unpinned=0

  # Find uses: lines with version tags but not SHA
  # SHA format: uses: owner/repo@sha256:... or uses: owner/repo@[a-f0-9]{40}
  while IFS= read -r line; do
    # Extract action reference
    local action
    action="$(echo "$line" | sed -n 's/.*uses:[[:space:]]*\([^@[:space:]]\+@[^[:space:]]\+\).*/\1/p')"

    if [[ -n "$action" ]]; then
      local version
      version="$(echo "$action" | cut -d@ -f2)"

      # Check if it's a tag version (v1, v2, main) vs SHA
      if [[ "$version" =~ ^v[0-9]|^main|^master|^latest ]]; then
        echo -e "  ${YELLOW}Unpinned action:${NC} $action"
        echo -e "    ${CYAN}Recommend:${NC} Pin to SHA for security"
        ((unpinned++)) || true
      fi
    fi
  done < <(grep "^.*uses:" "$file" || true)

  return $unpinned
}

check_permissions() {
  local file="$1"
  local warnings=0

  # Check for permissions declaration
  if ! grep -q "^permissions:" "$file"; then
    echo -e "  ${YELLOW}No 'permissions' declaration${NC}"
    echo -e "    ${CYAN}Recommend:${NC} Declare explicit permissions for security"
    ((warnings++)) || true
  fi

  return $warnings
}

check_common_issues() {
  local file="$1"
  local issues=0

  # Check for deprecated syntax
  if grep -q "set-env\|add-path" "$file"; then
    echo -e "  ${RED}Deprecated commands detected (set-env/add-path)${NC}"
    ((issues++)) || true
  fi

  # Check for checkout without persist-credentials=false (security)
  if grep -q "actions/checkout" "$file" && ! grep -q "persist-credentials: false" "$file"; then
    echo -e "  ${YELLOW}checkout without persist-credentials: false${NC}"
    echo -e "    ${CYAN}Recommend:${NC} Add persist-credentials: false for security"
  fi

  # Check for hardcoded secrets
  if grep -qE "(password|token|secret|key|api_key).*=.*['\"][^'$]*['\"]" "$file"; then
    echo -e "  ${RED}Possible hardcoded secret detected${NC}"
    ((issues++)) || true
  fi

  # Check for shell: bash missing in run steps
  if grep -q "run:" "$file" && ! grep -q "shell:" "$file"; then
    # This is often fine on Linux runners, but worth noting
    if $VERBOSE; then
      echo -e "  ${YELLOW}No explicit shell declaration in run steps${NC}"
    fi
  fi

  return $issues
}

# =============================================================================
# Fix common issues
# =============================================================================
fix_workflow() {
  local file="$1"

  echo -e "${CYAN}Attempting fixes for: $file${NC}"

  # Fix trailing whitespace
  if grep -q ' $' "$file"; then
    local tmp_file="$(mktemp)"
    sed 's/[[:space:]]*$//' "$file" > "$tmp_file"
    mv "$tmp_file" "$file"
    echo -e "  ${GREEN}Fixed:${NC} Removed trailing whitespace"
  fi

  # Fix tabs to spaces (2 spaces for YAML)
  if grep -qP '\t' "$file"; then
    # This is tricky - proper conversion needs care
    echo -e "  ${YELLOW}Warning:${NC} Tabs detected - manual fix recommended"
    echo "    YAML uses 2-space indentation"
  fi

  # Note: SHA pinning and permissions should be manual changes
  echo -e "  ${YELLOW}Note:${NC} SHA pinning and permissions require manual review"
}

# =============================================================================
# Main validation
# =============================================================================
validate_all_workflows() {
  echo -e "${CYAN}━━━ GitHub Actions Workflow Validation ━━━${NC}\n"

  local total_errors=0
  local total_warnings=0
  local files_checked=0

  # Check workflows directory
  if [[ ! -d "$WORKFLOWS_DIR" ]]; then
    echo -e "${RED}Error: Workflows directory not found: $WORKFLOWS_DIR${NC}"
    exit 1
  fi

  # Find workflow files
  local workflow_files
  workflow_files="$(find "$WORKFLOWS_DIR" -name "*.yml" -o -name "*.yaml" 2>/dev/null)"

  if [[ -z "$workflow_files" ]]; then
    echo -e "${YELLOW}No workflow files found${NC}"
    return 0
  fi

  # Validate each file
  while IFS= read -r file; do
    local filename
    filename="$(basename "$file")"
    files_checked=$((files_checked + 1))

    echo -e "\n${BLUE}Validating: $filename${NC}"
    echo -e "${GREEN}────────────────────────────────────${NC}"

    local file_errors=0
    local file_warnings=0

    # YAML syntax
    validate_yaml_syntax "$file" || ((file_errors++)) || true

    # Required fields
    check_required_fields "$file" || ((file_errors++)) || true

    # Action pinning (security)
    check_action_pinning "$file" || ((file_warnings++)) || true

    # Permissions
    check_permissions "$file" || ((file_warnings++)) || true

    # Common issues
    check_common_issues "$file" || ((file_errors++)) || true

    # Summary for file
    if [[ $file_errors -eq 0 && $file_warnings -eq 0 ]]; then
      echo -e "${GREEN}Passed${NC}"
    elif [[ $file_errors -eq 0 ]]; then
      echo -e "${YELLOW}Warnings: $file_warnings${NC}"
    else
      echo -e "${RED}Errors: $file_errors, Warnings: $file_warnings${NC}"
    fi

    total_errors=$((total_errors + file_errors))
    total_warnings=$((total_warnings + file_warnings))

    # Fix mode
    if $FIX_MODE && [[ $file_warnings -gt 0 || $file_errors -gt 0 ]]; then
      fix_workflow "$file"
    fi

  done <<< "$workflow_files"

  # Final summary
  echo -e "\n${CYAN}━━━ Summary ━━━${NC}"
  echo -e "  Files checked: $files_checked"
  echo -e "  ${RED}Errors:${NC} $total_errors"
  echo -e "  ${YELLOW}Warnings:${NC} $total_warnings"

  if [[ $total_errors -gt 0 ]]; then
    echo -e "\n${RED}Validation failed with $total_errors errors${NC}"

    if $CHECK_MODE; then
      exit 1
    else
      return 1
    fi
  elif [[ $total_warnings -gt 0 ]]; then
    echo -e "\n${YELLOW}Validation passed with $total_warnings warnings${NC}"
    echo "Review warnings and consider fixes for security best practices"
  else
    echo -e "\n${GREEN}All workflows validated successfully!${NC}"
  fi
}

# =============================================================================
# Main flow
# =============================================================================
cd "$REPO_ROOT"

# Default mode if no flags
if ! $CHECK_MODE && ! $FIX_MODE && ! $VERBOSE; then
  # Run basic validation
  validate_all_workflows
else
  validate_all_workflows
fi