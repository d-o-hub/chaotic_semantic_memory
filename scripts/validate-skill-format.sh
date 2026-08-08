#!/usr/bin/env bash
# =============================================================================
# validate-skill-format.sh - Fail-closed SKILL.md validation
# =============================================================================
# Usage: ./scripts/validate-skill-format.sh [--verbose]
#
# FAIL-CLOSED validator for .agents/skills/<name>/SKILL.md. Any violation
# makes the whole run exit non-zero; a broken skill can never pass silently.
#
# Checks (per skill):
#   1. Frontmatter parses: SKILL.md starts with '---', has a closing '---',
#      a non-empty 'name' field, and a non-empty 'description' field.
#   2. The 'name' field matches the skill directory name.
#   3. SKILL.md is at most MAX_SKILL_LINES (250) lines long.
#   4. Reference integrity (repo convention: reference files live in the
#      skill's references/ or reference/ directory and are pointed to from
#      the SKILL.md body):
#        a. Every `references/<file>` / `reference/<file>` mention in the
#           body must resolve to a real file -- in the skill's own
#           references/ (or reference/) directory first, then anywhere in
#           .agents/skills/ (cross-skill references, e.g. triz-solver ->
#           triz-analysis/references/principles.md, are legal).
#        b. Every file inside the skill's own references/ / reference/
#           directory MUST be mentioned in that skill's SKILL.md body
#           (no orphan reference files).
#   5. Optional per-skill check script: a skill may ship an executable
#      'check.sh' in its root or in scripts/. The runner executes it EXACTLY
#      ONCE and propagates its real exit status. Failures are never masked
#      (no `|| true`, no `|| :` around a skill script invocation).
#
# Runner contract (the behavior above, documented):
#   - Each skill's check script is invoked at most once per run, and a run
#     invokes it exactly once per skill that ships one.
#   - A non-zero exit status from the script fails the run (exit code 1),
#     even though static checks on remaining skills still complete so one
#     run reports every broken skill.
#
# Options:
#   --verbose   Show frontmatter/reference details per skill
#   --help      Show this help message
#
# Exit codes:
#   0 - All SKILL.md files valid
#   1 - Validation violations found (format, size, references, check.sh)
#   2 - Error occurred (bad usage, missing skills dir, no SKILL.md files)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"
MAX_SKILL_LINES=250

VERBOSE=false

for arg in "$@"; do
    case $arg in
        --verbose|-v) VERBOSE=true ;;
        --help|-h)
            cat << 'EOF'
Usage: validate-skill-format.sh [--verbose]

Fail-closed SKILL.md validation.

Checks (per skill):
  - Frontmatter parses (--- delimiters, non-empty name and description)
  - name matches the skill directory name
  - SKILL.md is at most 250 lines
  - references/ references in the body resolve to real files; files in the
    skill's references/ (or reference/) directory are all mentioned in its
    SKILL.md body
  - an executable check.sh in the skill root or scripts/ runs exactly once,
    and its real exit status is propagated (never masked)

Exit codes:
  0 - All SKILL.md files valid
  1 - Format issues found
  2 - Error occurred
EOF
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 2
            ;;
    esac
done

# Colors (portable across Linux/macOS)
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  SKILL.md Fail-Closed Validation${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

if [[ ! -d "${SKILLS_DIR}" ]]; then
    echo -e "${RED}✗ Skills directory not found: ${SKILLS_DIR}${NC}"
    exit 2
fi

# Counters
TOTAL=0
VALID=0
NO_FRONTMATTER=0
UNCLOSED_FRONTMATTER=0
MISSING_NAME=0
MISSING_DESC=0
NAME_MISMATCH=0
OVERLONG=0
DANGLING_REF=0
ORPHAN_REF=0
SCRIPT_RUN=0
SCRIPT_FAIL=0

# True if the given reference token resolves to a real file: the skill's own
# references/ or reference/ directory first, then any reference directory
# anywhere under .agents/skills/ (cross-skill references are legal).
resolve_ref() {
    local filename="$1"
    local candidate
    for candidate in \
        "${skill_dir}/references/${filename}" \
        "${skill_dir}/reference/${filename}"; do
        if [[ -f "${candidate}" ]]; then
            return 0
        fi
    done
    if find "${SKILLS_DIR}" -type f -path "*/reference*/${filename}" 2>/dev/null \
        | grep -q .; then
        return 0
    fi
    return 1
}

SKILL_FILES=$(find "${SKILLS_DIR}" -mindepth 2 -maxdepth 2 -name "SKILL.md" -type f | sort)

if [[ -z "${SKILL_FILES}" ]]; then
    echo -e "${RED}✗ No SKILL.md files found in ${SKILLS_DIR}${NC}"
    exit 2
fi

echo -e "${CYAN}→ Checking SKILL.md files...${NC}"
echo ""

for skill_file in ${SKILL_FILES}; do
    skill_dir=$(dirname "${skill_file}")
    skill_name_dir=$(basename "${skill_dir}")
    ((TOTAL++)) || true

    first_line=$(head -n 1 "${skill_file}")
    if [[ "${first_line}" != "---" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: missing frontmatter (no --- at start)"
        ((NO_FRONTMATTER++)) || true
        continue
    fi

    # Closing delimiter: first '---' line after line 1.
    closing_line=$(awk 'NR>1 && /^---$/ { print NR; exit }' "${skill_file}" || true)
    if [[ -z "${closing_line}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: unclosed frontmatter (no closing ---)"
        ((UNCLOSED_FRONTMATTER++)) || true
        continue
    fi

    frontmatter=$(sed -n "2,$((closing_line - 1))p" "${skill_file}")
    body=$(sed -n "$((closing_line + 1)),\$p" "${skill_file}")

    if [[ -z "${frontmatter}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: empty frontmatter block"
        ((UNCLOSED_FRONTMATTER++)) || true
        continue
    fi

    name_line=$(printf '%s\n' "${frontmatter}" | grep -E '^name:' || true)
    if [[ -z "${name_line}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: missing 'name' field"
        ((MISSING_NAME++)) || true
        continue
    fi

    skill_name=$(printf '%s\n' "${name_line}" | sed -E 's/^name:[[:space:]]*//' | tr -d '\042' | tr -d '\047' | tr -d ' ')
    if [[ -z "${skill_name}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: empty 'name' field"
        ((MISSING_NAME++)) || true
        continue
    fi

    desc_line=$(printf '%s\n' "${frontmatter}" | grep -E '^description:' || true)
    if [[ -z "${desc_line}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: missing 'description' field"
        ((MISSING_DESC++)) || true
        continue
    fi

    desc_value=$(printf '%s\n' "${desc_line}" | sed -E 's/^description:[[:space:]]*//')
    if [[ -z "${desc_value}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: empty 'description' field"
        ((MISSING_DESC++)) || true
        continue
    fi

    if [[ "${skill_name}" != "${skill_name_dir}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: name mismatch (frontmatter: '${skill_name}')"
        ((NAME_MISMATCH++)) || true
        continue
    fi

    # Size gate: at most MAX_SKILL_LINES lines in SKILL.md.
    loc=$(wc -l < "${skill_file}")
    if [[ "${loc}" -gt "${MAX_SKILL_LINES}" ]]; then
        echo -e "${RED}✗${NC} ${skill_name_dir}: SKILL.md is ${loc} lines (max ${MAX_SKILL_LINES})"
        ((OVERLONG++)) || true
    fi

    # References: every 'references/<file>' / 'reference/<file>' token in the
    # body must resolve to a real file.
    ref_tokens=$(printf '%s\n' "${body}" | grep -oE 'reference[s]?/[A-Za-z0-9._-]+' || true)
    if [[ -n "${ref_tokens}" ]]; then
        while IFS= read -r token; do
            filename="${token#*/}"
            if ! resolve_ref "${filename}"; then
                echo -e "${RED}✗${NC} ${skill_name_dir}: dangling reference '${token}'"
                ((DANGLING_REF++)) || true
            elif $VERBOSE; then
                echo "       reference: ${token} -> found"
            fi
        done <<< "${ref_tokens}"
    fi

    # References: every file in the skill's own references/ or reference/
    # directory must be mentioned in its own SKILL.md body (no orphans).
    for ref_dir in "${skill_dir}/references" "${skill_dir}/reference"; do
        if [[ ! -d "${ref_dir}" ]]; then
            continue
        fi
        while IFS= read -r ref_file; do
            ref_filename=$(basename "${ref_file}")
            if ! printf '%s\n' "${body}" | grep -qE "reference[s]?/${ref_filename}"; then
                echo -e "${RED}✗${NC} ${skill_name_dir}: reference file '$(basename "${ref_dir}")/${ref_filename}' is not mentioned in SKILL.md"
                ((ORPHAN_REF++)) || true
            fi
        done < <(find "${ref_dir}" -type f | sort)
    done

    # Optional per-skill check script: run EXACTLY ONCE, propagate exit status.
    check_script=""
    for candidate in "${skill_dir}/check.sh" "${skill_dir}/scripts/check.sh"; do
        if [[ -f "${candidate}" && -x "${candidate}" ]]; then
            check_script="${candidate}"
            break
        fi
    done
    if [[ -n "${check_script}" ]]; then
        ((SCRIPT_RUN++)) || true
        echo "       ${skill_name_dir}: running check script (${check_script#"${PROJECT_ROOT}"/})"
        if (cd "${PROJECT_ROOT}" && bash "${check_script}"); then
            echo -e "${GREEN}✓${NC} ${skill_name_dir}: check script passed"
        else
            rc=$?
            echo -e "${RED}✗${NC} ${skill_name_dir}: check script FAILED (exit ${rc})"
            ((SCRIPT_FAIL++)) || true
        fi
    fi

    echo -e "${GREEN}✓${NC} ${skill_name_dir}: frontmatter valid"
    if $VERBOSE; then
        echo "       Name: ${skill_name}"
        echo "       Description: ${desc_value}"
        echo "       SKILL.md: $(wc -l < "${skill_file}") lines"
    fi
    ((VALID++)) || true
done

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Total SKILL.md files:      ${TOTAL}"
echo "Valid:                     ${VALID}"
echo "Missing frontmatter:       ${NO_FRONTMATTER}"
echo "Unclosed frontmatter:      ${UNCLOSED_FRONTMATTER}"
echo "Missing name:              ${MISSING_NAME}"
echo "Missing description:       ${MISSING_DESC}"
echo "Name mismatch:             ${NAME_MISMATCH}"
echo "Over 250 lines:            ${OVERLONG}"
echo "Dangling references:       ${DANGLING_REF}"
echo "Orphan reference files:    ${ORPHAN_REF}"
echo "Check scripts executed:    ${SCRIPT_RUN} (exactly once per skill)"
echo "Check script failures:     ${SCRIPT_FAIL}"
echo ""

ISSUES=$((NO_FRONTMATTER + UNCLOSED_FRONTMATTER + MISSING_NAME + MISSING_DESC + NAME_MISMATCH + OVERLONG + DANGLING_REF + ORPHAN_REF + SCRIPT_FAIL))

if [[ "${ISSUES}" -gt 0 ]]; then
    echo -e "${RED}✗ Found ${ISSUES} validation violation(s)${NC}"
    echo ""
    echo "To fix frontmatter:"
    echo "  ---"
    echo "  name: <skill-name>"
    echo "  description: \"When to use this skill\""
    echo "  ---"
    echo ""
    echo "Reference convention: files live in <skill>/references/ (or "
    echo "reference/) and MUST be mentioned in the SKILL.md body; every "
    echo "reference/<file> mentioned MUST exist."
    echo ""
    echo "Check scripts: a skill may ship executable check.sh; it runs once"
    echo "and its exit status is final for the skill."
    exit 1
else
    echo -e "${GREEN}✓ All SKILL.md files pass fail-closed validation${NC}"
    exit 0
fi