#!/usr/bin/env bash
# =============================================================================
# validate-skill-refs.sh - Detect dead file references in SKILL.md files
# =============================================================================
# Checks ALL paths referenced in skills (src/, crates/, plans/, scripts/)
# against the actual filesystem. Catches stale paths after refactors.
#
# Usage: ./scripts/validate-skill-refs.sh
#
# Exit 0 = clean, Exit 1 = dead references found
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"

DEAD=0

# Extract paths that look like project file references from a SKILL.md
extract_project_paths() {
    local file="$1"
    # Backtick-quoted paths — skip if the containing line is inside a prose quote ("...")
    while IFS= read -r line; do
        # Skip lines where the backtick is inside a double-quoted string
        [[ "$line" =~ ^[[:space:]]*\" ]] && continue
        [[ "$line" =~ \"[^\"]*\`[^\"]*\" ]] && continue
        echo "$line" | grep -oE '`[^`]+`' | sed 's/^`//;s/`$//' \
            | grep -E '^\./|^src/|^crates/|^plans/|^scripts/|^tests/|^benches/|^benchmarks/|^\.github/' || true
    done < "$file"
    # Markdown link targets
    grep -oE '\]\([^)]+\)' "$file" | sed 's/^](//;s/)$//' \
        | grep -E '^\./|^src/|^crates/|^plans/|^scripts/|^tests/' || true
}

while IFS= read -r skill_file; do
    skill_name="$(basename "$(dirname "$skill_file")")"
    skill_dir="$(dirname "$skill_file")"

    while IFS= read -r ref_path; do
        [[ -z "$ref_path" ]] && continue
        # Strip trailing punctuation, anchors
        ref_path="${ref_path%%#*}"
        ref_path="${ref_path%,}"
        ref_path="${ref_path%.}"
        ref_path="${ref_path%;}"
        ref_path="${ref_path%:}"
        # Strip quotes
        ref_path="${ref_path#\"}"
        ref_path="${ref_path%\"}"
        ref_path="${ref_path#\'}"
        ref_path="${ref_path%\'}"
        # Skip glob patterns, env vars, placeholders, template tokens
        [[ "$ref_path" == *'*'* ]] && continue
        [[ "$ref_path" == *'$'* ]] && continue
        [[ "$ref_path" == *'<'* ]] && continue
        [[ "$ref_path" == *'{'* ]] && continue
        [[ "$ref_path" == *'N'*'N'*'-'* ]] && continue  # NNNN- template placeholders
        # Skip bare example paths (single filename like ./path.md with no real prefix)
        [[ "$ref_path" == "./path.md" ]] && continue
        # Resolve: try project root, then skill-local dir
        resolved="${PROJECT_ROOT}/${ref_path#./}"
        skill_local="${skill_dir}/${ref_path#./}"
        if [[ -e "$skill_local" ]] || [[ -d "$skill_local" ]]; then continue; fi
        if [[ ! -e "$resolved" ]] && [[ ! -d "$resolved" ]]; then
            echo "DEAD: ${skill_name}/SKILL.md → ${ref_path}"
            ((DEAD++)) || true
        fi
    done < <(extract_project_paths "$skill_file")

done < <(find "${SKILLS_DIR}" -mindepth 2 -maxdepth 2 -name "SKILL.md" -type f | sort)

echo ""
if [[ $DEAD -gt 0 ]]; then
    echo "Found ${DEAD} dead reference(s) across skills."
    echo "Run after workspace refactors to catch stale paths."
    exit 1
fi
echo "All skill file references resolve. Clean."
