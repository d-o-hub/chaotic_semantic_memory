#!/usr/bin/env bash
# =============================================================================
# setup-skills.sh - Create symlinks from .claude/skills/ → .agents/skills/
# =============================================================================
# Usage: ./scripts/setup-skills.sh [--force]
#
# Creates symlinks so multiple AI tools (Claude, Gemini, Qwen) can read the
# same canonical skills from .agents/skills/.
#
# Options:
#   --force    Remove existing directories/symlinks before creating new ones
#   --help     Show this help message
#
# Exit codes:
#   0 - Symlinks created successfully
#   1 - Error occurred (missing source, permission denied, etc.)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SOURCE_DIR="${PROJECT_ROOT}/.agents/skills"
TARGET_DIR="${PROJECT_ROOT}/.claude/skills"

FORCE=false

for arg in "$@"; do
    case $arg in
        --force) FORCE=true ;;
        --help|-h)
            cat << 'EOF'
Usage: setup-skills.sh [--force]

Create symlinks from .claude/skills/ → .agents/skills/

Options:
  --force    Remove existing directories/symlinks before creating
  --help     Show this help message

Description:
  This script enables multiple AI tools to share canonical skills by
  creating symlinks in .claude/skills/ pointing to .agents/skills/.

  Template workflow: .claude/skills/ → .agents/skills/ (canonical)

Exit codes:
  0 - Success
  1 - Error (missing source, permission denied, etc.)
EOF
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Colors ( portable across Linux/macOS )
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Skills Symlink Setup${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check source directory exists
if [[ ! -d "${SOURCE_DIR}" ]]; then
    echo -e "${RED}✗ Source directory not found: ${SOURCE_DIR}${NC}"
    echo ""
    echo "Create .agents/skills/ with skill directories first."
    echo "Each skill directory should contain a SKILL.md file."
    exit 1
fi

echo -e "${GREEN}✓${NC} Source directory: ${SOURCE_DIR}"

# Create target parent directory if missing
if [[ ! -d "${TARGET_DIR}" ]]; then
    mkdir -p "${TARGET_DIR}"
    echo -e "${GREEN}✓${NC} Created target directory: ${TARGET_DIR}"
fi

# Count skills
SKILL_COUNT=0
CREATED_COUNT=0
SKIPPED_COUNT=0
REMOVED_COUNT=0

# Get list of skills from source
SKILLS=$(find "${SOURCE_DIR}" -mindepth 1 -maxdepth 1 -type d | sort)

if [[ -z "${SKILLS}" ]]; then
    echo -e "${YELLOW}⚠ No skill directories found in ${SOURCE_DIR}${NC}"
    exit 0
fi

echo ""
echo -e "${CYAN}→ Processing skills...${NC}"
echo ""

for skill_path in ${SKILLS}; do
    skill_name=$(basename "${skill_path}")
    target_path="${TARGET_DIR}/${skill_name}"
    ((SKILL_COUNT++)) || true

    # Check if skill has SKILL.md
    if [[ ! -f "${skill_path}/SKILL.md" ]]; then
        echo -e "${YELLOW}⊘${NC} ${skill_name}: missing SKILL.md (skipped)"
        ((SKIPPED_COUNT++)) || true
        continue
    fi

    # Handle existing target
    if [[ -e "${target_path}" ]] || [[ -L "${target_path}" ]]; then
        if [[ -L "${target_path}" ]]; then
            # Existing symlink
            existing_target=$(readlink "${target_path}" 2>/dev/null || true)
            if [[ "${existing_target}" == "${skill_path}" ]]; then
                echo -e "${GREEN}✓${NC} ${skill_name}: symlink already correct"
                continue
            fi
            if $FORCE; then
                rm "${target_path}"
                echo -e "${YELLOW}↻${NC} ${skill_name}: removed old symlink"
                ((REMOVED_COUNT++)) || true
            else
                echo -e "${YELLOW}⊘${NC} ${skill_name}: symlink exists (use --force to replace)"
                ((SKIPPED_COUNT++)) || true
                continue
            fi
        elif [[ -d "${target_path}" ]]; then
            # Existing directory
            if $FORCE; then
                rm -rf "${target_path}"
                echo -e "${YELLOW}↻${NC} ${skill_name}: removed directory"
                ((REMOVED_COUNT++)) || true
            else
                echo -e "${YELLOW}⊘${NC} ${skill_name}: directory exists (use --force to replace)"
                ((SKIPPED_COUNT++)) || true
                continue
            fi
        else
            # Existing file
            if $FORCE; then
                rm "${target_path}"
                echo -e "${YELLOW}↻${NC} ${skill_name}: removed file"
                ((REMOVED_COUNT++)) || true
            else
                echo -e "${YELLOW}⊘${NC} ${skill_name}: file exists (use --force to replace)"
                ((SKIPPED_COUNT++)) || true
                continue
            fi
        fi
    fi

    # Create symlink
    ln -s "${skill_path}" "${target_path}"
    echo -e "${GREEN}✓${NC} ${skill_name}: symlink created → ${skill_path}"
    ((CREATED_COUNT++)) || true
done

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Skills found:     ${SKILL_COUNT}"
echo "Symlinks created: ${CREATED_COUNT}"
echo "Removed:          ${REMOVED_COUNT}"
echo "Skipped:          ${SKIPPED_COUNT}"
echo ""

if [[ "${CREATED_COUNT}" -eq 0 ]] && [[ "${SKIPPED_COUNT}" -gt 0 ]]; then
    echo -e "${YELLOW}⚠ No new symlinks created. Use --force to replace existing items.${NC}"
    exit 0
elif [[ "${CREATED_COUNT}" -gt 0 ]]; then
    echo -e "${GREEN}✓ Symlink setup complete${NC}"
    exit 0
else
    echo -e "${GREEN}✓ All symlinks already correct${NC}"
    exit 0
fi