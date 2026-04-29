#!/usr/bin/env bash
# =============================================================================
# validate-links.sh - Check reference links in SKILL.md files
# =============================================================================
# Usage: ./scripts/validate-links.sh [--verbose]
#
# Validates that links in SKILL.md files point to existing files:
#   - @file.md style references
#   - [text](./path.md) markdown links
#   - Relative paths within the skill directory
#
# Options:
#   --verbose    Show all checked links (not just broken ones)
#   --help       Show this help message
#
# Exit codes:
#   0 - All links valid
#   1 - Broken links found
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
Usage: validate-links.sh [--verbose]

Check reference links in SKILL.md files

Options:
  --verbose    Show all checked links (not just broken ones)
  --help       Show this help message

Description:
  Validates that links in SKILL.md files point to existing files:
  - @file.md style references (e.g., @AGENTS.md, @references/pattern.md)
  - [text](./path.md) markdown links
  - Relative paths within the skill directory

  Skips:
  - External URLs (http://, https://)
  - Anchor links (#section)
  - npm packages (@scope/package)
  - Version tags (@v1.0.0)

Exit codes:
  0 - All links valid
  1 - Broken links found
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
echo -e "${CYAN}  SKILL.md Link Validation${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check skills directory exists
if [[ ! -d "${SKILLS_DIR}" ]]; then
    echo -e "${RED}✗ Skills directory not found: ${SKILLS_DIR}${NC}"
    exit 2
fi

# Counters
TOTAL_LINKS=0
VALID_LINKS=0
BROKEN_LINKS=0
SKILLS_CHECKED=0

# Helper function to resolve and check a link
check_link() {
    local link="$1"
    local source_file="$2"
    local source_dir
    source_dir=$(dirname "${source_file}")

    # Resolve the target path relative to source directory
    local target_path

    # Handle @file references
    if [[ "${link}" =~ ^@(.*)$ ]]; then
        local after_at="${BASH_REMATCH[1]}"
        target_path="${source_dir}/${after_at}"
    # Handle relative paths (./path or ../path or just path)
    elif [[ "${link}" =~ ^\. ]] || [[ "${link}" =~ ^[a-zA-Z] ]]; then
        target_path="${source_dir}/${link}"
    else
        return 0  # Skip unsupported link formats
    fi

    # Normalize path (handles .. and .)
    # Using cd for portability across Linux/macOS
    if [[ -d "$(dirname "${target_path}")" ]]; then
        target_path=$(cd "$(dirname "${target_path}")" && pwd)/$(basename "${target_path}")
    else
        return 1
    fi

    # Check if file exists
    [[ -f "${target_path}" ]]
}

# Find all SKILL.md files
SKILL_FILES=$(find "${SKILLS_DIR}" -mindepth 2 -maxdepth 2 -name "SKILL.md" -type f | sort)

if [[ -z "${SKILL_FILES}" ]]; then
    echo -e "${YELLOW}⚠ No SKILL.md files found in ${SKILLS_DIR}${NC}"
    exit 0
fi

echo -e "${CYAN}→ Checking links in SKILL.md files...${NC}"
echo ""

for skill_file in ${SKILL_FILES}; do
    skill_dir=$(basename "$(dirname "${skill_file}")")
    ((SKILLS_CHECKED++)) || true

    skill_broken=0
    skill_total=0

    # Extract @file references
    # Skip npm packages, version tags, emails, GitHub mentions
    while IFS= read -r link; do
        # Skip npm packages (@scope/package)
        if [[ "${link}" =~ ^@[^/]+/ ]]; then continue; fi
        # Skip version tags (@v1.0.0)
        if [[ "${link}" =~ ^@v[0-9] ]]; then continue; fi
        # Skip short mentions (likely GitHub @mentions)
        after_at="${link#@}"
        if [[ ! "${after_at}" =~ \. ]] && [[ ${#after_at} -lt 15 ]]; then continue; fi
        # Skip common non-file patterns
        if [[ "${link}" == "@-mentions" ]]; then continue; fi

        ((TOTAL_LINKS++)) || true
        ((skill_total++)) || true

        if check_link "${link}" "${skill_file}"; then
            ((VALID_LINKS++)) || true
            if $VERBOSE; then
                echo -e "${GREEN}  ✓${NC} ${skill_dir}: ${link}"
            fi
        else
            ((BROKEN_LINKS++)) || true
            ((skill_broken++)) || true
            echo -e "${RED}  ✗${NC} ${skill_dir}: ${link}"
        fi
    done < <(grep -oE '@[a-zA-Z0-9_./-]+' "${skill_file}" 2>/dev/null | sort -u || true)

    # Extract [text](path) style links
    while IFS= read -r link; do
        # Skip external URLs and anchor links
        if [[ "${link}" =~ ^https?:// ]]; then continue; fi
        if [[ "${link}" =~ ^# ]]; then continue; fi

        ((TOTAL_LINKS++)) || true
        ((skill_total++)) || true

        if check_link "${link}" "${skill_file}"; then
            ((VALID_LINKS++)) || true
            if $VERBOSE; then
                echo -e "${GREEN}  ✓${NC} ${skill_dir}: [link]( ${link})"
            fi
        else
            ((BROKEN_LINKS++)) || true
            ((skill_broken++)) || true
            echo -e "${RED}  ✗${NC} ${skill_dir}: ${link}"
        fi
    done < <(grep -oE '\]\([^)]+\)' "${skill_file}" 2>/dev/null | sed 's/\](\(.*\))/\1/' | sort -u || true)

    # Report per-skill summary if verbose
    if $VERBOSE && [[ "${skill_total}" -gt 0 ]]; then
        if [[ "${skill_broken}" -eq 0 ]]; then
            echo -e "${GREEN}✓${NC} ${skill_dir}: all ${skill_total} links valid"
        else
            echo -e "${RED}✗${NC} ${skill_dir}: ${skill_broken}/${skill_total} broken"
        fi
    fi
done

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Skills checked:   ${SKILLS_CHECKED}"
echo "Total links:      ${TOTAL_LINKS}"
echo "Valid links:      ${VALID_LINKS}"
echo "Broken links:     ${BROKEN_LINKS}"
echo ""

if [[ "${BROKEN_LINKS}" -gt 0 ]]; then
    echo -e "${RED}✗ Found ${BROKEN_LINKS} broken link(s)${NC}"
    echo ""
    echo "To fix:"
    echo "  1. Ensure referenced files exist in skill directory"
    echo "  2. Check relative paths are correct"
    echo "  3. Use ./path.md or @path.md for relative references"
    exit 1
else
    echo -e "${GREEN}✓ All links valid${NC}"
    exit 0
fi