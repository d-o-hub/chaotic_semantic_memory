#!/usr/bin/env bash
# sync-skills.sh — Auto-update all skill references from .agents/skills/ source of truth
#
# Scans .agents/skills/*/SKILL.md frontmatter, then updates:
#   1. AGENTS.md — categorized skills list + count
#   2. CLAUDE.md — specialist skills table
#   3. plans/SWARM_COORDINATION.md — combined agents table (if exists)
#   4. Runs scripts/generate-agents.sh to regenerate .opencode/agents/
#
# Usage:
#   ./scripts/sync-skills.sh           # Dry-run: report what would change
#   ./scripts/sync-skills.sh --apply   # Apply changes
#   ./scripts/sync-skills.sh --check   # Exit 1 if changes needed (CI mode)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly PROJECT_ROOT
readonly SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"

MODE="dry-run"
for arg in "$@"; do
    case "$arg" in
        --apply) MODE="apply" ;;
        --check)  MODE="check" ;;
    esac
done

# ── Category assignment ──────────────────────────────────────────────
# Default: "Core". Overridden by naming prefix or hardcoded exceptions.
declare -A CATEGORY_EXCEPTIONS=(
    ["analysis-swarm"]="Swarm"
    ["jules-orchestration"]="Automation"
)

categorize() {
    local name="$1"
    # Check exceptions first
    if [[ -n "${CATEGORY_EXCEPTIONS[$name]:-}" ]]; then
        echo "${CATEGORY_EXCEPTIONS[$name]}"
        return
    fi
    # Naming convention heuristics
    case "$name" in
        swarm-*)     echo "Swarm" ;;
        triz-*)      echo "TRIZ" ;;
        self-*|skill-*) echo "Automation" ;;
        learn|task-decomposition|shell-script-quality) echo "Workflow" ;;
        *)           echo "Core" ;;
    esac
}

# ── Extract frontmatter from a SKILL.md ──────────────────────────────
parse_skill() {
    local file="$1"
    local name desc
    name=$(grep -m1 '^name:' "$file" | sed 's/^name: *//')
    desc=$(grep -m1 '^description:' "$file" \
        | sed -E 's/^description: *"//; s/"$//; s/^description: *//')
    if [[ -z "$name" ]]; then
        echo "WARNING: ${file} missing 'name:' frontmatter — skipping" >&2
        return 1
    fi
    echo "${name}|${desc}"
}

# ── Helper: convert comma-space-separated string to line-per-name ─────
# Needed because "IFS=', ' read -ra" doesn't always split cleanly
split_names() {
    local input="$1"
    printf '%s\n' "$input" | tr ',' '\n' | sed 's/^ *//; s/ *$//'
}

# ── Main logic ───────────────────────────────────────────────────────
main() {
    echo "=== Scanning skills ==="
    local skills=() names=()
    while IFS= read -r -d '' file; do
        local line
        if line=$(parse_skill "$file"); then
            skills+=("$line")
            names+=("${line%%|*}")
        fi
    done < <(find "$SKILLS_DIR" -maxdepth 2 -name 'SKILL.md' -print0 | sort -z)

    local count=${#skills[@]}
    echo "Found: ${count} skills"

    if [[ $count -eq 0 ]]; then
        echo "WARNING: No skills found in ${SKILLS_DIR} — nothing to sync"
        exit 1
    fi

    # Group by category
    declare -A groups
    for skill_line in "${skills[@]}"; do
        local name="${skill_line%%|*}"
        local cat
        cat=$(categorize "$name")
        groups["$cat"]="${groups[$cat]:-}${name}, "
    done

    # Report
    echo ""
    echo "=== Categories ==="
    for cat in Core Swarm Workflow Automation TRIZ; do
        local skill_list="${groups[$cat]:-}"
        if [[ -n "$skill_list" ]]; then
            skill_list="${skill_list%, }"
            local cat_count
            cat_count=$(echo "$skill_list" | tr ',' '\n' | wc -l)
            echo "  ${cat}: ${skill_list} (${cat_count})"
        fi
    done

    # ── 1. Update AGENTS.md ──────────────────────────────────────────
    echo ""
    echo "=== Updating AGENTS.md ==="

    # Build the new skills section
    local new_section
    new_section="## Skills (${count} Total)"$'\n'
    for cat in Core Swarm Workflow Automation TRIZ; do
        local skill_list="${groups[$cat]:-}"
        if [[ -n "$skill_list" ]]; then
            skill_list="${skill_list%, }"
            new_section+="**${cat}**: ${skill_list}"$'\n'
        fi
    done

    local agents_file="${PROJECT_ROOT}/AGENTS.md"
    local skills_start skills_end
    skills_start=$(grep -n '^## Skills ' "$agents_file" | head -1 | cut -d: -f1)

    if [[ -z "$skills_start" ]]; then
        echo "ERROR: Could not find '## Skills' section in AGENTS.md"
        exit 1
    fi

    skills_end=$(tail -n +"$((skills_start + 1))" "$agents_file" \
        | grep -n '^## ' | head -1 | cut -d: -f1)
    if [[ -n "$skills_end" ]]; then
        skills_end=$((skills_start + skills_end - 1))
    else
        skills_end=$(wc -l < "$agents_file")
    fi

    local section_lines=$((skills_end - skills_start + 1))
    local new_lines
    new_lines=$(printf '%s' "$new_section" | wc -l)

    if [[ "$MODE" == "apply" ]]; then
        {
            head -n $((skills_start - 1)) "$agents_file"
            printf '%s' "$new_section"
            tail -n +$((skills_end + 1)) "$agents_file"
        } > "${agents_file}.tmp"
        mv "${agents_file}.tmp" "$agents_file"
        echo "  Updated AGENTS.md skills section"
    else
        echo "  [dry-run] Would update AGENTS.md: ${section_lines} lines → ${new_lines} lines"
    fi

    # ── 2. Update CLAUDE.md ──────────────────────────────────────────
    echo ""
    echo "=== Updating CLAUDE.md ==="
    local claude_file="${PROJECT_ROOT}/CLAUDE.md"

    # Build description lookup (associative array to avoid arithmetic evaluation of hyphens)
    declare -A skill_desc_map
    for skill_line in "${skills[@]}"; do
        local sname="${skill_line%%|*}"
        local sdesc="${skill_line#*|}"
        # Strip pipe characters that would break markdown tables
        sdesc="${sdesc//|/ }"
        skill_desc_map["$sname"]="$sdesc"
    done

    # Get Core and Swarm skill name lists
    local core_names_str="${groups[Core]:-}"
    core_names_str="${core_names_str%, }"
    local swarm_names_str="${groups[Swarm]:-}"
    swarm_names_str="${swarm_names_str%, }"

    # Build Core table rows (top 6)
    local core_rows=""
    local ccount=0
    while IFS= read -r sname; do
        [[ -z "$sname" ]] && continue
        [[ $ccount -ge 6 ]] && break
        local sdesc="${skill_desc_map["$sname"]:-}"
        sdesc=$(echo "$sdesc" | cut -c1-50 | sed 's/\.[^.]*$//')
        core_rows+="| \`${sname}\` | ${sdesc} |"$'\n'
        ccount=$((ccount + 1))
    done < <(split_names "$core_names_str")

    # Build Swarm table rows
    local swarm_rows=""
    while IFS= read -r sname; do
        [[ -z "$sname" ]] && continue
        local sdesc="${skill_desc_map["$sname"]:-}"
        sdesc=$(echo "$sdesc" | cut -c1-50 | sed 's/\.[^.]*$//')
        swarm_rows+="| \`${sname}\` | ${sdesc} |"$'\n'
    done < <(split_names "$swarm_names_str")

    # The full replacement block wrapped in sentinel comments for robust section detection
    local new_claude_table
    new_claude_table=$(cat <<EOF
<!-- SKILLS_TABLE_START -->
## Specialist Skills

Loaded on-demand via \`/skill-name\` or auto-triggered by description.

| Core Skills | Purpose |
|-------------|---------|
${core_rows}
| Swarm Skills | Focus |
|--------------|-------|
${swarm_rows}
<!-- SKILLS_TABLE_END -->
EOF
)

    # Find sentinel-bounded section
    local claude_start="" claude_end=""
    claude_start=$(grep -n '^<!-- SKILLS_TABLE_START -->' "$claude_file" | head -1 | cut -d: -f1)
    claude_end=$(grep -n '^<!-- SKILLS_TABLE_END -->' "$claude_file" | head -1 | cut -d: -f1)

    if [[ -z "$claude_start" || -z "$claude_end" ]]; then
        echo "  WARNING: Sentinel markers not found in CLAUDE.md — run --apply with markers present first"
    elif [[ "$MODE" == "apply" ]]; then
        {
            head -n $((claude_start - 1)) "$claude_file"
            printf '%s' "$new_claude_table"
            tail -n +$((claude_end + 1)) "$claude_file"
        } > "${claude_file}.tmp"
        mv "${claude_file}.tmp" "$claude_file"
        echo "  Updated CLAUDE.md skills table"
    else
        echo "  [dry-run] Would update CLAUDE.md specialist skills table"
    fi

    # ── 3. Update SWARM_COORDINATION.md (if exists) ──────────────────
    local swarm_file="${PROJECT_ROOT}/plans/SWARM_COORDINATION.md"
    if [[ -f "$swarm_file" ]]; then
        echo ""
        echo "=== Updating SWARM_COORDINATION.md ==="
        if [[ "$MODE" == "apply" ]]; then
            # Collect swarm skill names
            local s_names=()
            while IFS= read -r sname; do
                [[ -n "$sname" ]] && s_names+=("$sname")
            done < <(split_names "$swarm_names_str")

            local has_swarm_adv=no has_swarm_obs=no has_analysis=no
            for sname in "${s_names[@]}"; do
                case "$sname" in
                    swarm-advanced-features) has_swarm_adv=yes ;;
                    swarm-observability) has_swarm_obs=yes ;;
                    analysis-swarm) has_analysis=yes ;;
                esac
            done

            local swarm_skills=""
            [[ "$has_swarm_adv" == yes ]] && swarm_skills+=" + swarm-advanced-features"
            [[ "$has_swarm_obs" == yes ]] && swarm_skills+=" + swarm-observability"
            [[ "$has_analysis" == yes ]] && swarm_skills+=" + analysis-swarm"
            swarm_skills="${swarm_skills# + }"

            sed -i \
                "s/| @perf | .* | Performance |/| @perf | benchmarking-perf + debugging-reservoir | Performance |/" \
                "$swarm_file"
            sed -i \
                "s/| @test | .* | Testing |/| @test | testing-validation | Testing |/" \
                "$swarm_file"
            echo "  Updated SWARM_COORDINATION.md"
        else
            echo "  [dry-run] Would update SWARM_COORDINATION.md combined agents table"
        fi
    fi

    # ── 4. Regenerate .opencode/ agents ──────────────────────────────
    echo ""
    echo "=== Regenerating .opencode/ agents ==="
    if [[ -x "${SCRIPT_DIR}/generate-agents.sh" ]]; then
        if [[ "$MODE" == "apply" ]]; then
            "${SCRIPT_DIR}/generate-agents.sh"
            echo "  Regenerated .opencode/agents/"
        else
            echo "  [dry-run] Would run: scripts/generate-agents.sh"
        fi
    else
        echo "  SKIP: scripts/generate-agents.sh not found or not executable"
    fi

    # ── Summary ──────────────────────────────────────────────────────
    echo ""
    echo "=== Summary ==="
    echo "Skills: ${count} total"
    for cat in Core Swarm Workflow Automation TRIZ; do
        local skill_list="${groups[$cat]:-}"
        if [[ -n "$skill_list" ]]; then
            skill_list="${skill_list%, }"
            local cat_count
            cat_count=$(echo "$skill_list" | tr ',' '\n' | wc -l)
            echo "  ${cat}: ${cat_count}"
        fi
    done

    if [[ "$MODE" == "check" ]]; then
        # CI mode: compare what we'd generate against current files, exit 1 if stale
        local stale=0

        # Check AGENTS.md: normalize trailing newlines before comparison
        local current_agents_skills expected_agents
        current_agents_skills=$(sed -n "${skills_start},${skills_end}p" "$agents_file")
        # Strip trailing newlines from both sides
        expected_agents="${new_section%"${new_section##*[!
]}"}"
        local current_agents_trimmed="${current_agents_skills%"${current_agents_skills##*[!
]}"}"
        if [[ "$current_agents_trimmed" != "$expected_agents" ]]; then
            echo "  STALE: AGENTS.md skills section needs update"
            stale=1
        fi

        # Check CLAUDE.md: normalize trailing newlines
        if [[ -n "${claude_start:-}" ]]; then
            local current_claude_table
            current_claude_table=$(sed -n "${claude_start},${claude_end}p" "$claude_file")
            local expected_claude="${new_claude_table%"${new_claude_table##*[!
]}"}"
            local current_claude_trimmed="${current_claude_table%"${current_claude_table##*[!
]}"}"
            if [[ "$current_claude_trimmed" != "$expected_claude" ]]; then
                echo "  STALE: CLAUDE.md specialist skills table needs update"
                stale=1
            fi
        fi

        if [[ $stale -eq 1 ]]; then
            echo ""
            echo "CHECK FAILED: Run ./scripts/sync-skills.sh --apply to sync."
            exit 1
        fi
        echo ""
        echo "Check passed: all references are up to date."
    elif [[ "$MODE" == "dry-run" ]]; then
        echo ""
        echo "Dry-run complete. Use --apply to apply changes."
    fi
}

main
