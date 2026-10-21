#!/usr/bin/env bash
# =============================================================================
# check-shared-conventions.sh - Validate .agents/context/shared-conventions.md
# =============================================================================
# Usage: ./scripts/check-shared-conventions.sh [--target FILE] [--root DIR]
#
# Checks:
#   1. Target context file exists.
#   2. File is concise (line count <= 150).
#   3. File does not duplicate skill catalogs (no '| Skill | Trigger |' tables
#      or copied SKILL.md frontmatter/trees).
#   4. All linked internal paths exist on disk.
#
# Exit codes:
#   0 - All checks passed
#   1 - Validation failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TARGET_FILE="${PROJECT_ROOT}/.agents/context/shared-conventions.md"
MAX_LINES=150

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            TARGET_FILE="$2"
            shift 2
            ;;
        --root)
            PROJECT_ROOT="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [--target FILE] [--root DIR]"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

if [[ ! -f "${TARGET_FILE}" ]]; then
    echo "❌ Error: Target context file not found: ${TARGET_FILE}" >&2
    exit 1
fi

ERRORS=0

# 1. Line count check (conciseness)
LINE_COUNT=$(wc -l < "${TARGET_FILE}")
if [[ "${LINE_COUNT}" -gt "${MAX_LINES}" ]]; then
    echo "❌ Error: Shared conventions file exceeds ${MAX_LINES} lines (${LINE_COUNT} lines)." >&2
    ERRORS=$((ERRORS + 1))
fi

# 2. Skill catalog anti-duplication check
if grep -qE '^\|[[:space:]]*Skill[[:space:]]*\|[[:space:]]*Trigger[[:space:]]*\|' "${TARGET_FILE}"; then
    echo "❌ Error: Shared conventions file contains a duplicated skill catalog table." >&2
    ERRORS=$((ERRORS + 1))
fi

if grep -qE '^[[:space:]]*triggers:' "${TARGET_FILE}"; then
    echo "❌ Error: Shared conventions file contains copied skill YAML frontmatter." >&2
    ERRORS=$((ERRORS + 1))
fi

# 3. Path existence check
TARGET_DIR="$(dirname "${TARGET_FILE}")"

# Helper to check link existence
check_path() {
    local link="$1"
    local target_path=""

    # Strip anchors (#...)
    link="${link%%#*}"
    if [[ -z "$link" ]]; then
        return 0
    fi

    if [[ "$link" =~ ^@(.*)$ ]]; then
        local raw_path="${BASH_REMATCH[1]}"
        if [[ -e "${PROJECT_ROOT}/${raw_path}" ]]; then
            target_path="${PROJECT_ROOT}/${raw_path}"
        elif [[ -e "${TARGET_DIR}/${raw_path}" ]]; then
            target_path="${TARGET_DIR}/${raw_path}"
        else
            target_path="${PROJECT_ROOT}/${raw_path}"
        fi
    elif [[ "$link" =~ ^\./ ]] || [[ "$link" =~ ^\.\./ ]] || [[ "$link" =~ ^[a-zA-Z0-9_] ]]; then
        if [[ "$link" =~ ^https?:// ]]; then
            return 0
        fi
        if [[ -e "${TARGET_DIR}/${link}" ]]; then
            target_path="${TARGET_DIR}/${link}"
        elif [[ -e "${PROJECT_ROOT}/${link}" ]]; then
            target_path="${PROJECT_ROOT}/${link}"
        else
            target_path="${TARGET_DIR}/${link}"
        fi
    else
        return 0
    fi

    if [[ ! -e "$target_path" ]]; then
        echo "❌ Broken link '$link' in ${TARGET_FILE} (resolved to '${target_path}')" >&2
        return 1
    fi
    return 0
}

# Extract @path links
while IFS= read -r link; do
    [[ -z "$link" ]] && continue
    # Skip npm scopes, semantic tags, domain names, example quotes
    [[ "$link" =~ ^@[^/]+/ ]] && continue
    [[ "$link" =~ ^@v?[0-9]+\.[0-9]+ ]] && continue
    [[ "$link" =~ ^@[0-9a-fA-F]{7,40}$ ]] && continue
    [[ "$link" == "@file.md" || "$link" == "@path.md" || "$link" == "@imports" ]] && continue

    raw_link="${link#@}"
    # Require dot or slash in file reference
    if [[ "$raw_link" =~ \. || "$raw_link" =~ / ]]; then
        if ! check_path "$link"; then
            ERRORS=$((ERRORS + 1))
        fi
    fi
done < <(grep -oE '@[a-zA-Z0-9_./-]+' "${TARGET_FILE}" 2>/dev/null | sort -u || true)

# Extract [text](path) links
while IFS= read -r link; do
    [[ -z "$link" ]] && continue
    [[ "$link" =~ ^https?:// || "$link" =~ ^# ]] && continue
    [[ "$link" == "./path.md" || "$link" == "file.md" ]] && continue

    if ! check_path "$link"; then
        ERRORS=$((ERRORS + 1))
    fi
done < <(grep -oE '\]\([^)]+\)' "${TARGET_FILE}" 2>/dev/null | sed 's/\](\(.*\))/\1/' | sort -u || true)

if [[ "${ERRORS}" -gt 0 ]]; then
    echo "❌ check-shared-conventions failed with ${ERRORS} issue(s)" >&2
    exit 1
fi

echo "✅ Shared conventions validation passed for ${TARGET_FILE}"
exit 0
