#!/usr/bin/env bash
# =============================================================================
# validate-workflows.sh - YAML validation for GitHub Actions
# =============================================================================
# Usage: scripts/validate-workflows.sh [--check] [--fix]
#
# Validates GitHub Actions workflow files:
#   - YAML validation, classified into distinct outcomes: syntax error (does not
#     parse) / lint finding (yamllint -f parsable, split by [error] vs [warning])
#     / advisory (this script's own heuristics, never a parse failure)
#   - Required fields, action SHA pinning and permissions (advisories)
#
# Configuration comes from the repo-root .yamllint profile (extends: relaxed,
# line-length raised to 200). Without it the gate scored unfixable 80-column
# noise from pinned `uses:` lines as repository defects (issue #841).
#
# Flags:
#   --check         Validate all workflows (exit 1 on errors)
#   --fix           Attempt to fix common issues
#   --verbose       Show detailed validation output
#   --help          Show this help message
#
# Exit contract for --check: 1 if and only if at least one syntax error or
# error-level finding exists. Warnings, advisories and parser-less SKIPPED files
# never fail it — but a skipped file is reported loudly, never as a pass.
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
  1. YAML validation, reported as three distinct classes: syntax error (the file
     does not parse), lint finding (yamllint error/warning level, from the
     repo-root .yamllint profile), advisory (this script's own heuristics)
  2. Required fields (name, on, jobs)
  3. Action SHA pinning and permissions declarations (advisories)

Exit contract: --check exits 1 only for a syntax error or an error-level
finding; warnings, advisories and files with no parser available are reported
loudly but never fail it.

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
# Per-file counters. The old code returned a finding count through `return
# $errors` and the caller collapsed any non-zero value with
# `validate_yaml_syntax "$file" || ((file_errors++))`, so a file carrying three
# error findings was reported as "Errors: 1" — the summary counted categories,
# never findings. These globals hold real counts and are reset per file.
YAML_SYNTAX_ERRORS=0
YAML_LINT_ERRORS=0
YAML_LINT_WARNINGS=0
YAML_SKIPPED=0
FOUND_ERRORS=0
FOUND_ADVISORIES=0

# Reporting helpers (yaml_count / finding_error / finding_advisory /
# print_findings) live in scripts/lib/yaml-findings.sh. Sourced fail-closed: a
# missing helper must not degrade into "no findings printed, gate green".
if [[ -f "${SCRIPT_DIR}/lib/yaml-findings.sh" ]]; then
  # shellcheck source=lib/yaml-findings.sh
  source "${SCRIPT_DIR}/lib/yaml-findings.sh"
else
  echo -e "${RED}Error: ${SCRIPT_DIR}/lib/yaml-findings.sh missing — cannot classify YAML findings${NC}"
  exit 1
fi

validate_yaml_syntax() {
  local file="$1"
  local out="" rc=0 findings="" syntax="" lint_err="" lint_warn="" parse_err=""

  YAML_SYNTAX_ERRORS=0
  YAML_LINT_ERRORS=0
  YAML_LINT_WARNINGS=0
  YAML_SKIPPED=0

  if command -v yamllint >/dev/null 2>&1; then
    # -f parsable, not the human format: the classification below depends on the
    # machine-readable `[error]` / `[warning]` level and the `(rule)` suffix.
    out="$(yamllint -f parsable "$file" 2>&1)" || rc=$?
    findings="$(printf '%s\n' "$out" | grep -E ': \[(error|warning)\] ' || true)"

    if [[ -z "$findings" ]]; then
      if [[ "$rc" -ne 0 ]]; then
        # yamllint failed without naming a single finding (bad profile, crash,
        # unreadable file): no verdict. Loud, counted as skipped, and NOT
        # reported as a YAML error the repository does not have.
        echo -e "  ${YELLOW}SKIPPED — NOT A PASS: yamllint exited ${rc} without a finding${NC}"
        printf '%s\n' "$out" | tail -n 3 | sed 's/^/    /'
        YAML_SKIPPED=1
      fi
      return 0
    fi

    syntax="$(printf '%s\n' "$findings" | grep '\[error\].*syntax error' || true)"
    lint_err="$(printf '%s\n' "$findings" | grep '\[error\]' | grep -v 'syntax error' || true)"
    lint_warn="$(printf '%s\n' "$findings" | grep '\[warning\]' || true)"

    YAML_SYNTAX_ERRORS="$(yaml_count "$syntax")"
    YAML_LINT_ERRORS="$(yaml_count "$lint_err")"
    YAML_LINT_WARNINGS="$(yaml_count "$lint_warn")"

    print_findings "YAML syntax error — the file does not parse" "$RED" all "$syntax"
    print_findings "YAML lint finding, error level (NOT a parse failure)" "$RED" all "$lint_err"
    print_findings "YAML lint finding, warning level (advisory)" "$YELLOW" 5 "$lint_warn"
  elif command -v python3 >/dev/null 2>&1 && python3 -c 'import yaml' >/dev/null 2>&1; then
    # The `import yaml` probe is load-bearing: without it a box that has python3
    # but no PyYAML made the fallback exit 1 with "No module named 'yaml'", which
    # used to be printed as a YAML syntax error for every single file.
    parse_err="$(python3 -c 'import sys, yaml; yaml.safe_load(open(sys.argv[1]))' \
      "$file" 2>&1)" || rc=$?
    if [[ "$rc" -ne 0 ]]; then
      YAML_SYNTAX_ERRORS=1
      echo -e "  ${RED}YAML syntax error — the file does not parse${NC} (PyYAML fallback)"
      printf '%s\n' "$parse_err" | tail -n 3 | sed 's/^/    /'
    fi
  else
    # No parser at all. Never a pass, never a failure: say so, then keep the
    # grep-level checks as clearly-labelled advisories only. The old branch also
    # ran `grep -n '^  [^ ]' | grep -v '^  [a-z]'` and counted it as an error —
    # grep -n prefixes line numbers, so nothing ever began with two spaces and
    # every indented line was reported as a false indentation defect.
    YAML_SKIPPED=1
    echo -e "  ${YELLOW}SKIPPED — NOT A PASS: neither yamllint nor python3+PyYAML is${NC}"
    echo -e "  ${YELLOW}available, so this file was never parsed. Grep-level advisories:${NC}"
    if grep -q ' $' "$file"; then
      finding_advisory "trailing whitespace (unverified by a parser)"
    fi
    if grep -qP '\t' "$file"; then
      finding_advisory "tab characters — YAML requires spaces (unverified by a parser)"
    fi
  fi

  if [[ $((YAML_SYNTAX_ERRORS + YAML_LINT_ERRORS)) -gt 0 ]]; then
    return 1
  fi
  return 0
}

check_required_fields() {
  local file="$1"

  # Check for 'name:' field
  if ! grep -q "^name:" "$file"; then
    finding_error "missing 'name' field"
  fi

  # Check for 'on:' trigger
  if ! grep -q "^on:" "$file"; then
    finding_error "missing 'on' trigger definition"
  fi

  # Check for 'jobs:' section
  if ! grep -q "^jobs:" "$file"; then
    finding_error "missing 'jobs' section"
  fi

  return 0
}

check_action_pinning() {
  local file="$1"

  # Find uses: lines with version tags but not SHA
  # SHA format: uses: owner/repo@sha256:... or uses: owner/repo@[a-f0-9]{40}
  while IFS= read -r line; do
    # Extract action reference
    local action
    action="$(echo "$line" | grep -oP 'uses:\s*\K[^@]+@[^\s]+')"

    if [[ -n "$action" ]]; then
      local version
      version="$(echo "$action" | cut -d@ -f2)"

      # Check if it's a tag version (v1, v2, main) vs SHA
      if [[ "$version" =~ ^v[0-9]|^main|^master|^latest ]]; then
        finding_advisory "unpinned action: $action — pin to SHA for security"
      fi
    fi
  done < <(grep "^.*uses:" "$file" || true)

  return 0
}

check_permissions() {
  local file="$1"

  # Check for permissions declaration
  if ! grep -q "^permissions:" "$file"; then
    finding_advisory "no 'permissions' declaration — declare explicit permissions"
  fi

  return 0
}

check_common_issues() {
  local file="$1"

  # Check for deprecated syntax. The removed workflow commands are written as
  # `::set-env` / `::add-path`; the old bare `set-env\|add-path` pattern matched
  # release.yml:184 (`add-paths: llms.txt,llms-full.txt`, an action INPUT key)
  # and failed --check on an otherwise clean tree.
  if grep -qE ':(set-env|add-path)' "$file"; then
    finding_error "deprecated workflow commands (::set-env / ::add-path) detected"
  fi

  # Check for checkout without persist-credentials=false (security)
  if grep -q "actions/checkout" "$file" && ! grep -q "persist-credentials: false" "$file"; then
    finding_advisory "checkout without persist-credentials: false — add it for security"
  fi

  # Check for hardcoded secrets
  if grep -qE "(password|token|secret|key|api_key).*=.*['\"][^'$]*['\"]" "$file"; then
    finding_error "possible hardcoded secret detected"
  fi

  # Check for shell: bash missing in run steps
  if grep -q "run:" "$file" && ! grep -q "shell:" "$file"; then
    # This is often fine on Linux runners, but worth noting
    if $VERBOSE; then
      finding_advisory "no explicit shell declaration in run steps"
    fi
  fi

  return 0
}

# =============================================================================
# Fix common issues
# =============================================================================
fix_workflow() {
  local file="$1"

  echo -e "${CYAN}Attempting fixes for: $file${NC}"

  # Fix trailing whitespace
  if grep -q ' $' "$file"; then
    sed -i 's/[[:space:]]*$//' "$file"
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
  local total_advisories=0
  local total_skipped=0
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

    FOUND_ERRORS=0
    FOUND_ADVISORIES=0

    # yamllint echoes the path it was handed, so an absolute argument makes every
    # finding a 100-character path nobody can copy into an editor. The script has
    # already cd'd to REPO_ROOT, so the repo-relative form is valid everywhere.
    local rel_file="${file#"$REPO_ROOT"/}"

    # YAML: parse failures, error-level lint findings, warning-level findings and
    # no-verdict skips are four different things and are counted separately.
    validate_yaml_syntax "$rel_file" || true

    # Required fields, pinning, permissions, common issues — these fill FOUND_*
    # rather than returning counts the caller would have to collapse.
    check_required_fields "$rel_file"
    check_action_pinning "$rel_file"
    check_permissions "$rel_file"
    check_common_issues "$rel_file"

    local file_errors=$((YAML_SYNTAX_ERRORS + YAML_LINT_ERRORS + FOUND_ERRORS))
    local file_warnings=$YAML_LINT_WARNINGS
    local file_advisories=$FOUND_ADVISORIES

    # Summary for file, split by class so a lint finding can never be read as a
    # parse failure and an advisory can never be read as either.
    if [[ $file_errors -gt 0 ]]; then
      echo -e "${RED}Errors: ${file_errors} (syntax ${YAML_SYNTAX_ERRORS}," \
        "lint ${YAML_LINT_ERRORS}, structural ${FOUND_ERRORS})," \
        "Warnings: ${file_warnings}, Advisories: ${file_advisories}${NC}"
    elif [[ $YAML_SKIPPED -gt 0 ]]; then
      echo -e "${YELLOW}SKIPPED — NOT A PASS (no YAML parser verdict), Advisories: ${file_advisories}${NC}"
    elif [[ $file_warnings -gt 0 || $file_advisories -gt 0 ]]; then
      echo -e "${YELLOW}Warnings: ${file_warnings}, Advisories: ${file_advisories}${NC}"
    else
      echo -e "${GREEN}Passed${NC}"
    fi

    total_errors=$((total_errors + file_errors))
    total_warnings=$((total_warnings + file_warnings))
    total_advisories=$((total_advisories + file_advisories))
    total_skipped=$((total_skipped + YAML_SKIPPED))

    # Fix mode
    if $FIX_MODE && [[ $file_advisories -gt 0 || $file_warnings -gt 0 || $file_errors -gt 0 ]]; then
      fix_workflow "$file"
    fi

  done <<< "$workflow_files"

  # Final summary
  echo -e "\n${CYAN}━━━ Summary ━━━${NC}"
  echo -e "  Files checked: $files_checked"
  echo -e "  ${RED}Errors (syntax + error-level lint + structural):${NC} $total_errors"
  echo -e "  ${YELLOW}Lint warnings:${NC} $total_warnings"
  echo -e "  ${YELLOW}Advisories (this script's own heuristics):${NC} $total_advisories"
  if [[ $total_skipped -gt 0 ]]; then
    echo -e "  ${YELLOW}Files with NO YAML verdict (SKIPPED — NOT A PASS):${NC} $total_skipped"
  fi

  if [[ $total_errors -gt 0 ]]; then
    echo -e "\n${RED}Validation failed with $total_errors errors${NC}"

    if $CHECK_MODE; then
      exit 1
    else
      return 1
    fi
  elif [[ $total_skipped -gt 0 ]]; then
    # A skip never fails the gate and never passes it either: it says plainly
    # that the YAML of these files was not checked at all.
    echo -e "\n${YELLOW}NOT VALIDATED: $total_skipped file(s) had no YAML parser" \
      "(yamllint or python3+PyYAML) — install one to actually check them.${NC}"
  elif [[ $total_warnings -gt 0 || $total_advisories -gt 0 ]]; then
    echo -e "\n${YELLOW}Validation passed with $total_warnings warnings" \
      "and $total_advisories advisories${NC}"
    echo "Review advisories and consider fixes for security best practices"
  else
    echo -e "\n${GREEN}All workflows validated successfully!${NC}"
  fi
}

# =============================================================================
# Main flow
# =============================================================================
cd "$REPO_ROOT"
validate_all_workflows