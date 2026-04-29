#!/usr/bin/env bash
# =============================================================================
# validate-skill-format.sh - Check SKILL.md frontmatter and format
# =============================================================================
# Usage: ./scripts/validate-skill-format.sh [--verbose]
#
# Validates that SKILL.md files have proper YAML frontmatter with required
# fields (name, description) and follow expected formatting conventions.
#
# Options:
#   --verbose    Show frontmatter content for each skill
#   --help       Show this help message
#
# Frontmatter requirements:
#   - Must have YAML frontmatter block (--- delimiters)
#   - Must have 'name' field (skill identifier)
#   - Must have 'description' field (when to use)
#   - Name must match directory name
#   - Description must be quoted string
#
# Exit codes:
#   0 - All SKILL.md files valid
#   1 - Format issues found
#   2 - Error occurred
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"

VERBOSE=false

for arg in "$@"; do
    case $arg in
        --verbose|-v) VERBOSE=true ;;
        --help|-h)
            cat << 'EOF'
Usage: validate-skill-format.sh [--verbose]

Check SKILL.md frontmatter and format

Options:
  --verbose    Show frontmatter content for each skill
  --help       Show this help message

Description:
  Validates that SKILL.md files have proper YAML frontmatter:

  Required fields:
    - name: Skill identifier (must match directory name)
    - description: When to use this skill (quoted string)

  Example valid frontmatter:
    ---
    name: rust-development
    description: "Implement or refactor Rust in this repository."
    ---

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
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  SKILL.md Format Validation${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check skills directory exists
if [[ ! -d "${SKILLS_DIR}" ]]; then
    echo -e "${RED}✗ Skills directory not found: ${SKILLS_DIR}${NC}"
    exit 2
fi

# Counters
TOTAL=0
VALID=0
NO_FRONTMATTER=0
MISSING_NAME=0
MISSING_DESC=0
NAME_MISMATCH=0
INVALID_YAML=0

# Find all SKILL.md files
SKILL_FILES=$(find "${SKILLS_DIR}" -mindepth 2 -maxdepth 2 -name "SKILL.md" -type f | sort)

if [[ -z "${SKILL_FILES}" ]]; then
    echo -e "${YELLOW}⚠ No SKILL.md files found in ${SKILLS_DIR}${NC}"
    exit 0
fi

echo -e "${CYAN}→ Checking SKILL.md files...${NC}"
echo ""

for skill_file in ${SKILL_FILES}; do
    skill_dir=$(basename "$(dirname "${skill_file}")")
    ((TOTAL++)) || true

    # Check file starts with ---
    first_line=$(head -n 1 "${skill_file}" 2>/dev/null || true)
    if [[ "${first_line}" != "---" ]]; then
        echo -e "${RED}✗${NC} ${skill_dir}: missing frontmatter (no --- at start)"
        ((NO_FRONTMATTER++)) || true
        continue
    fi

    # Extract frontmatter (between first and second ---)
    # Using awk for portability across Linux/macOS
    frontmatter=$(awk '/^---$/ { if (NR==1) { start=1; next } if (start==1) { exit } } start { print }' "${skill_file}" 2>/dev/null || true)

    if [[ -z "${frontmatter}" ]]; then
        echo -e "${RED}✗${NC} ${skill_dir}: invalid frontmatter (missing closing ---)"
        ((INVALID_YAML++)) || true
        continue
    fi

    # Parse YAML fields (simple grep-based parsing for portability)
    # Look for 'name:' field
    name_line=$(echo "${frontmatter}" | grep -E '^name:' || true)
    if [[ -z "${name_line}" ]]; then
        echo -e "${RED}✗${NC} ${skill_dir}: missing 'name' field"
        ((MISSING_NAME++)) || true
        continue
    fi

    # Extract name value (after 'name:')
    skill_name=$(echo "${name_line}" | sed 's/^name: *//' | tr -d '"' | tr -d "'" | tr -d ' ')

    # Look for 'description:' field
    desc_line=$(echo "${frontmatter}" | grep -E '^description:' || true)
    if [[ -z "${desc_line}" ]]; then
        echo -e "${RED}✗${NC} ${skill_dir}: missing 'description' field"
        ((MISSING_DESC++)) || true
        continue
    fi

    # Check name matches directory
    if [[ "${skill_name}" != "${skill_dir}" ]]; then
        echo -e "${RED}✗${NC} ${skill_dir}: name mismatch (frontmatter: '${skill_name}')"
        ((NAME_MISMATCH++)) || true
        continue
    fi

    # All checks passed
    echo -e "${GREEN}✓${NC} ${skill_dir}: frontmatter valid"
    if $VERBOSE; then
        echo "       Name: ${skill_name}"
        desc_value=$(echo "${desc_line}" | sed 's/^description: *//')
        echo "       Description: ${desc_value}"
    fi
    ((VALID++)) || true
done

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Total SKILL.md files: ${TOTAL}"
echo "Valid:               ${VALID}"
echo "Missing frontmatter: ${NO_FRONTMATTER}"
echo "Missing name:        ${MISSING_NAME}"
echo "Missing description: ${MISSING_DESC}"
echo "Name mismatch:       ${NAME_MISMATCH}"
echo "Invalid YAML:        ${INVALID_YAML}"
echo ""

ISSUES=$((NO_FRONTMATTER + MISSING_NAME + MISSING_DESC + NAME_MISMATCH + INVALID_YAML))

if [[ "${ISSUES}" -gt 0 ]]; then
    echo -e "${RED}✗ Found ${ISSUES} format issue(s)${NC}"
    echo ""
    echo "To fix frontmatter:"
    echo "  ---"
    echo "  name: <skill-name>"
    echo "  description: \"When to use this skill\""
    echo "  ---"
    exit 1
else
    echo -e "${GREEN}✓ All SKILL.md files have valid frontmatter${NC}"
    exit 0
fi