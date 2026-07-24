#!/usr/bin/env bash
# =============================================================================
# validate-skill-catalog.sh - Keep CATALOG.md in sync with disk
# =============================================================================
# Compares skill directories on disk with entries in CATALOG.md.
# Catches: skills deleted but still listed, skills added but not cataloged,
# count mismatch in header.
#
# Usage: ./scripts/validate-skill-catalog.sh [--fix]
#   --fix: regenerate the count line (non-destructive)
#
# Exit 0 = in sync, Exit 1 = drift detected
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"
CATALOG="${SKILLS_DIR}/CATALOG.md"

if [[ ! -f "$CATALOG" ]]; then
    echo "ERROR: ${CATALOG} not found"
    exit 2
fi

FIX=false
[[ "${1:-}" == "--fix" ]] && FIX=true

ISSUES=0

# Skills on disk (directories with SKILL.md)
DISK_SKILLS=()
while IFS= read -r d; do
    DISK_SKILLS+=("$(basename "$d")")
done < <(find "$SKILLS_DIR" -mindepth 1 -maxdepth 1 -type d | sort)

# Skills listed in CATALOG.md (bold entries)
CATALOG_SKILLS=()
while IFS= read -r entry; do
    # Extract skill name from **name**: pattern
    name=$(echo "$entry" | grep -oP '\*\*\K[^*]+' | head -1)
    [[ -n "$name" ]] && CATALOG_SKILLS+=("$name")
done < <(grep -E '^\- \*\*' "$CATALOG")

# Check for skills on disk but not in catalog
for skill in "${DISK_SKILLS[@]}"; do
    if ! printf '%s\n' "${CATALOG_SKILLS[@]}" | grep -qx "$skill"; then
        echo "MISSING from CATALOG: ${skill} (exists on disk)"
        ((ISSUES++)) || true
    fi
done

# Check for catalog entries with no disk directory
for skill in "${CATALOG_SKILLS[@]}"; do
    if [[ ! -d "${SKILLS_DIR}/${skill}" ]]; then
        echo "STALE in CATALOG: ${skill} (no directory on disk)"
        ((ISSUES++)) || true
    fi
done

# Check count in header
DISK_COUNT=${#DISK_SKILLS[@]}
HEADER_COUNT=$(grep -oE '[0-9]+ skills' "$CATALOG" | head -1 | grep -oE '[0-9]+')
if [[ "${HEADER_COUNT:-0}" -ne "$DISK_COUNT" ]]; then
    echo "COUNT MISMATCH: header says ${HEADER_COUNT}, disk has ${DISK_COUNT}"
    if $FIX; then
        sed -i "s/${HEADER_COUNT} skills/${DISK_COUNT} skills/" "$CATALOG"
        echo "  → Fixed count to ${DISK_COUNT}"
    else
        ((ISSUES++)) || true
    fi
fi

echo ""
if [[ $ISSUES -gt 0 ]]; then
    echo "${ISSUES} catalog drift issue(s). Run with --fix or update CATALOG.md manually."
    exit 1
fi
echo "CATALOG.md is in sync with disk (${DISK_COUNT} skills)."
