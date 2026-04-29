#!/usr/bin/env bash
# =============================================================================
# validate-skills.sh - Verify skill symlinks are intact
# =============================================================================
# Usage: ./scripts/validate-skills.sh [--verbose]
#
# Validates that symlinks in .claude/skills/ point to valid directories in
# .agents/skills/ and that each target has a SKILL.md file.
#
# Options:
#   --verbose    Show detailed output for each skill
#   --help       Show this help message
#
# Exit codes:
#   0 - All symlinks valid
#   1 - Invalid/broken symlinks found
#   2 - Error occurred
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${PROJECT_ROOT}/.claude/skills"
SOURCE_DIR="${PROJECT_ROOT}/.agents/skills"

VERBOSE=false

for arg in "$@"; do
    case $arg in
        --verbose|-v) VERBOSE=true ;;
        --help|-h)
            cat << 'EOF'
Usage: validate-skills.sh [--verbose]

Verify skill symlinks are intact

Options:
  --verbose    Show detailed output for each skill
  --help       Show this help message

Description:
  Validates that:
  1. .claude/skills/ entries are symlinks (not directories/files)
  2. Symlinks point to valid directories in .agents/skills/
  3. Target directories contain SKILL.md files

Exit codes:
  0 - All symlinks valid
  1 - Invalid/broken symlinks found
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
echo -e "${CYAN}  Skills Symlink Validation${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check target directory exists
if [[ ! -d "${TARGET_DIR}" ]]; then
    echo -e "${RED}✗ Target directory not found: ${TARGET_DIR}${NC}"
    echo ""
    echo "Run setup-skills.sh to create symlinks first."
    exit 2
fi

# Counters
TOTAL=0
VALID=0
NOT_SYMLINK=0
BROKEN=0
MISSING_SKILL_MD=0

# Get all entries in target directory
ENTRIES=$(find "${TARGET_DIR}" -mindepth 1 -maxdepth 1 | sort)

if [[ -z "${ENTRIES}" ]]; then
    echo -e "${YELLOW}⚠ No skills found in ${TARGET_DIR}${NC}"
    exit 0
fi

echo -e "${CYAN}→ Checking symlinks...${NC}"
echo ""

for entry_path in ${ENTRIES}; do
    entry_name=$(basename "${entry_path}")
    ((TOTAL++)) || true

    # Check if it's a symlink
    if [[ ! -L "${entry_path}" ]]; then
        echo -e "${RED}✗${NC} ${entry_name}: not a symlink"
        if $VERBOSE; then
            if [[ -d "${entry_path}" ]]; then
                echo "       Type: directory"
            elif [[ -f "${entry_path}" ]]; then
                echo "       Type: file"
            else
                echo "       Type: unknown"
            fi
        fi
        ((NOT_SYMLINK++)) || true
        continue
    fi

    # Get symlink target
    link_target=$(readlink "${entry_path}" 2>/dev/null || true)

    if [[ -z "${link_target}" ]]; then
        echo -e "${RED}✗${NC} ${entry_name}: cannot read symlink target"
        ((BROKEN++)) || true
        continue
    fi

    # Check if target exists and is a directory
    if [[ ! -d "${entry_path}" ]]; then
        echo -e "${RED}✗${NC} ${entry_name}: broken symlink (target not found)"
        if $VERBOSE; then
            echo "       Target: ${link_target}"
        fi
        ((BROKEN++)) || true
        continue
    fi

    # Check if target is in .agents/skills/ (optional strict check)
    if $VERBOSE; then
        # Resolve to absolute path for comparison
        resolved_target=$(cd "$(dirname "${entry_path}")" && cd "$(dirname "${link_target}")" && pwd)/$(basename "${link_target}")
        if [[ "${resolved_target}" != "${SOURCE_DIR}/${entry_name}" ]]; then
            echo -e "${YELLOW}⚠${NC} ${entry_name}: symlink points outside .agents/skills/"
            echo "       Target: ${resolved_target}"
            echo "       Expected: ${SOURCE_DIR}/${entry_name}"
        fi
    fi

    # Check for SKILL.md
    if [[ ! -f "${entry_path}/SKILL.md" ]]; then
        echo -e "${RED}✗${NC} ${entry_name}: missing SKILL.md in target"
        ((MISSING_SKILL_MD++)) || true
        continue
    fi

    # All checks passed
    echo -e "${GREEN}✓${NC} ${entry_name}: symlink valid"
    if $VERBOSE; then
        echo "       Target: ${link_target}"
        echo "       SKILL.md: present"
    fi
    ((VALID++)) || true
done

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Total entries:        ${TOTAL}"
echo "Valid symlinks:       ${VALID}"
echo "Not symlinks:         ${NOT_SYMLINK}"
echo "Broken symlinks:      ${BROKEN}"
echo "Missing SKILL.md:     ${MISSING_SKILL_MD}"
echo ""

ISSUES=$((NOT_SYMLINK + BROKEN + MISSING_SKILL_MD))

if [[ "${ISSUES}" -gt 0 ]]; then
    echo -e "${RED}✗ Found ${ISSUES} invalid symlink(s)${NC}"
    echo ""
    echo "To fix:"
    echo "  1. Run setup-skills.sh --force to recreate symlinks"
    echo "  2. Ensure each .agents/skills/<name>/ has a SKILL.md file"
    exit 1
else
    echo -e "${GREEN}✓ All symlinks valid${NC}"
    exit 0
fi