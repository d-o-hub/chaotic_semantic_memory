#!/usr/bin/env bash
# =============================================================================
# lib/yaml-findings.sh — reporting helpers for scripts/validate-workflows.sh
# =============================================================================
# Extracted (issue #841) so the subject stays near the repo's source-size
# budget: the classification itself deliberately remains in
# scripts/validate-workflows.sh, only the printing/counting lives here.
#
# Sourced, not executed. Requires the caller's colour variables (RED/GREEN/
# YELLOW/BLUE/NC), the VERBOSE flag, and the per-file FOUND_ERRORS /
# FOUND_ADVISORIES counters.
# =============================================================================

# yaml_count <captured-text> — number of non-empty lines, 0 for an empty string.
yaml_count() {
  local text="$1"
  if [[ -z "$text" ]]; then
    echo 0
  else
    printf '%s\n' "$text" | grep -c .
  fi
}

# finding_error / finding_advisory <message> — label the two non-YAML classes
# differently from a parse failure, and count them as findings.
finding_error() {
  echo -e "  ${RED}structural error: $1${NC}"
  FOUND_ERRORS=$((FOUND_ERRORS + 1))
}

finding_advisory() {
  echo -e "  ${YELLOW}advisory: $1${NC}"
  FOUND_ADVISORIES=$((FOUND_ADVISORIES + 1))
}

# print_findings <label> <colour> <limit> <text> — one indented line per
# finding, so the output carries the file:line:col the reader needs. Error-class
# findings are always shown in full; warnings are capped so ci.yml's 16
# indentation warnings cannot bury the actionable ones.
print_findings() {
  local label="$1" colour="$2" limit="$3" text="$4"
  local total
  total="$(yaml_count "$text")"
  if [[ "$total" -eq 0 ]]; then
    return 0
  fi
  echo -e "  ${colour}${label} (${total})${NC}"
  if $VERBOSE || [[ "$limit" == "all" ]] || [[ "$total" -le "$limit" ]]; then
    printf '%s\n' "$text" | sed 's/^/    /'
  else
    printf '%s\n' "$text" | head -n "$limit" | sed 's/^/    /'
    echo -e "    ${BLUE}... and $((total - limit)) more (rerun with --verbose)${NC}"
  fi
}
