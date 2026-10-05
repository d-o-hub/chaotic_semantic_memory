#!/usr/bin/env bash
# gen-skill-catalog.sh - render .agents/skills/CATALOG.md from the filesystem.
#
# WHY (issue #828, GOAP action `generate_skill_catalog_and_agent_context`)
# `CATALOG.md` was hand-typed and had silently drifted: it announced "32 skills."
# while `git ls-tree -r --name-only origin/main -- .agents/skills | grep -c
# 'SKILL.md$'` says 33, and the single missing row was `pr-roast-triage` — the
# skill `AGENTS.md` tells every agent to load for the mandatory "roast every
# PR/issue before implementing or merging" rule (Phase 1 step 5, Phase 5 step 17,
# Core Rule 9). `git grep -l CATALOG origin/main -- scripts .github .githooks`
# returns nothing: no script produced the file and no gate read it, so nothing
# could notice. This script makes the filesystem the only source of the skill set.
#
# WHAT COMES FROM WHERE
#   - skill set, row order, count line -> directory listing + YAML front matter
#   - each row's trigger text          -> that SKILL.md's `description:`
#   - section membership               -> CATEGORY_SPEC below: the one input that
#     is not on disk, because `name` and `description` are the only front-matter
#     keys in all 33 skills and editing SKILL.md is out of scope. A skill absent
#     from CATEGORY_SPEC is still catalogued, under `Unclassified`, so the
#     document can under-explain a skill but can never omit one or print a count
#     that disagrees with the tree.
#
# DELIBERATELY NOT DUPLICATED HERE
# SKILL.md conformance (front matter present, `name`/`description` non-empty,
# `name` equal to its directory, <= 250 lines) belongs to
# scripts/validate-skill-format.sh, already wired into scripts/validate.sh and
# .github/workflows/ci.yml. This script re-checks only what it must in order to
# render safely: a skill directory with no SKILL.md (invisible to a
# `find -name SKILL.md` inventory, which is how the other gate enumerates), an
# empty name/description, or two skills claiming one name.
#
# Determinism: LC_ALL=C, rows sorted by skill name inside each section, sections
# in CATEGORY_SPEC order, so re-running is byte-stable. No network, no cargo, and
# no write to the working tree unless --out points into it.
#
# Usage:
#   scripts/gen-skill-catalog.sh                        # rewrite the committed catalog
#   scripts/gen-skill-catalog.sh --root DIR --out FILE   # render elsewhere (gate, fixture)
# Drift gate: scripts/check-skill-catalog.sh
# Negative fixture: scripts/test-skill-catalog-gate.sh

set -euo pipefail
export LC_ALL=C

# Section order == emission order. Membership mirrors the section headings of the
# hand-written catalog this script replaces, with `pr-roast-triage` joining
# `Workflow` as AGENTS.md already groups it.
CATEGORY_SPEC=(
    'Core adr-creation benchmarking-perf debugging-reservoir dist-channel-selection github-ci-guardrails git-workflow goap-orchestrator goap-planning memory-lifecycle-verification npm-trusted-publishers release-management rust-development skill-memory-internal testing-validation turso-memory-verification'
    'Swarm analysis-swarm swarm-advanced-features swarm-observability swarm-performance swarm-testing-quality'
    'Workflow jules-orchestration learn pr-roast-triage shell-script-quality task-decomposition'
    'Automation codacy iterative-refinement self-fix-loop skill-creator skill-evaluator'
    'TRIZ triz-analysis triz-solver'
    'Visualization drawio'
)

readonly UNCLASSIFIED="Unclassified"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${SCRIPT_DIR}/.."
SKILLS_REL=".agents/skills"
OUT_FILE=""

usage() {
    cat <<'USAGE'
Usage: scripts/gen-skill-catalog.sh [--root DIR] [--out FILE]

  --root DIR  repository root holding .agents/skills/ (default: this script's parent)
  --out FILE  where to write the catalog (default: DIR/.agents/skills/CATALOG.md)

Exit codes:
  0  catalog rendered
  1  a skill directory has no SKILL.md, its front matter has no non-empty `name`
     or `description`, two skills report the same name, or no skills were found
  2  bad usage
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            [[ $# -ge 2 ]] || { echo "gen-skill-catalog: --root needs a value" >&2; exit 2; }
            ROOT="$2"
            shift 2
            ;;
        --out)
            [[ $# -ge 2 ]] || { echo "gen-skill-catalog: --out needs a value" >&2; exit 2; }
            OUT_FILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf 'gen-skill-catalog: unknown argument: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

[[ -d "${ROOT}" ]] || { printf 'gen-skill-catalog: not a directory: %s\n' "${ROOT}" >&2; exit 1; }
SKILLS_DIR="${ROOT}/${SKILLS_REL}"
[[ -d "${SKILLS_DIR}" ]] || { printf 'gen-skill-catalog: no skills directory: %s\n' "${SKILLS_DIR}" >&2; exit 1; }
[[ -n "${OUT_FILE}" ]] || OUT_FILE="${SKILLS_DIR}/CATALOG.md"

# --- front-matter parsing (same idiom as scripts/validate-skill-format.sh) ----

# Front matter body between the first and second `---` lines.
extract_frontmatter() {
    awk '
        /^---[[:space:]]*$/ {
            if (seen == 0) { seen = 1; next }
            if (seen == 1) { exit }
        }
        seen == 1 { print }
    ' "$1"
}

# Single-line `key: value`. Prints the trimmed value with one layer of matching
# quotes stripped, or nothing at all.
yaml_field() {
    local frontmatter="$1" key="$2" line raw
    line="$(printf '%s\n' "${frontmatter}" | grep -E "^${key}:" | head -n 1 || true)"
    [[ -z "${line}" ]] && return 0
    raw="${line#"${key}:"}"
    raw="${raw#"${raw%%[![:space:]]*}"}"
    raw="${raw%"${raw##*[![:space:]]}"}"
    if [[ "${raw}" =~ ^\"(.*)\"$ ]]; then
        raw="${BASH_REMATCH[1]}"
    elif [[ "${raw}" =~ ^\'(.*)\'$ ]]; then
        raw="${BASH_REMATCH[1]}"
    fi
    printf '%s' "${raw}"
}

# --- section map -------------------------------------------------------------

declare -A SECTION_OF=()
SECTION_ORDER=()
names=()

build_sections() {
    local spec member
    local -a parts
    for spec in "${CATEGORY_SPEC[@]}"; do
        read -r -a parts <<< "${spec}"
        SECTION_ORDER+=("${parts[0]}")
        for member in "${parts[@]:1}"; do
            SECTION_OF["${member}"]="${parts[0]}"
        done
    done
}

build_sections

# --- discovery ---------------------------------------------------------------

skill_dirs=()
while IFS= read -r dir; do
    [[ -n "${dir}" ]] && skill_dirs+=("${dir}")
done < <(find "${SKILLS_DIR}" -mindepth 1 -maxdepth 1 -type d -print | sort)

if [[ ${#skill_dirs[@]} -eq 0 ]]; then
    printf 'gen-skill-catalog: no skill directories under %s\n' "${SKILLS_DIR}" >&2
    exit 1
fi

errors=0
declare -A NAME_SEEN=()
declare -A DESC_OF=()
declare -A ROW_SECTION=()

for dir in "${skill_dirs[@]}"; do
    rel="${dir#"${ROOT}/"}"
    skill_file="${dir}/SKILL.md"
    if [[ ! -f "${skill_file}" ]]; then
        printf 'ERROR: skill directory without SKILL.md: %s\n' "${rel}" >&2
        errors=$((errors + 1))
        continue
    fi

    frontmatter="$(extract_frontmatter "${skill_file}")"
    skill_name="$(yaml_field "${frontmatter}" name)"
    if [[ -z "${skill_name}" ]]; then
        printf "ERROR: no front-matter 'name:' in %s\n" "${rel}/SKILL.md" >&2
        errors=$((errors + 1))
        continue
    fi

    skill_desc="$(yaml_field "${frontmatter}" description)"
    if [[ -z "${skill_desc}" ]]; then
        printf "ERROR: no front-matter 'description:' in %s\n" "${rel}/SKILL.md" >&2
        errors=$((errors + 1))
        continue
    fi

    if [[ -n "${NAME_SEEN[${skill_name}]:-}" ]]; then
        printf 'ERROR: duplicate skill name "%s" (second occurrence under %s)\n' \
            "${skill_name}" "${rel}" >&2
        errors=$((errors + 1))
        continue
    fi
    NAME_SEEN["${skill_name}"]=1

    # A bare `|` would split the markdown cell; escape it instead of dropping it.
    skill_desc="${skill_desc//|/\\|}"

    names+=("${skill_name}")
    DESC_OF["${skill_name}"]="${skill_desc}"
    ROW_SECTION["${skill_name}"]="${SECTION_OF[${skill_name}]:-${UNCLASSIFIED}}"
done

if [[ "${errors}" -gt 0 ]]; then
    printf 'gen-skill-catalog: %d skill(s) unrenderable - catalog NOT written\n' "${errors}" >&2
    exit 1
fi

if [[ ${#names[@]} -eq 0 ]]; then
    printf 'gen-skill-catalog: nothing renderable under %s\n' "${SKILLS_DIR}" >&2
    exit 1
fi

# Rows sorted by skill name, independent of directory order.
sorted_names=()
while IFS= read -r n; do
    [[ -n "${n}" ]] && sorted_names+=("${n}")
done < <(printf '%s\n' "${names[@]}" | sort)

# `Unclassified` earns a heading only when at least one skill actually needs it.
unclassified_used=0
for n in "${names[@]}"; do
    [[ "${ROW_SECTION[${n}]}" == "${UNCLASSIFIED}" ]] && unclassified_used=1
done
[[ "${unclassified_used}" -eq 1 ]] && SECTION_ORDER+=("${UNCLASSIFIED}")

# --- render ------------------------------------------------------------------

render_catalog() {
    local section n count
    printf '# Skill Catalog\n\n'
    printf '%s skills. Load with `skill` tool when task matches trigger.\n\n' \
        "${#sorted_names[@]}"
    printf 'Generated by `scripts/gen-skill-catalog.sh` from the tree: the count, the\n'
    printf 'sections and every row come from the YAML front matter of the matching\n'
    printf '`.agents/skills/*/SKILL.md`, never from hand-typed table rows. Re-run the\n'
    printf 'script after adding, removing or renaming a skill. SKILL.md conformance is\n'
    printf 'gated by `scripts/validate-skill-format.sh`; catalog drift by\n'
    printf '`scripts/check-skill-catalog.sh`.\n'

    for section in "${SECTION_ORDER[@]}"; do
        count=0
        for n in "${sorted_names[@]}"; do
            [[ "${ROW_SECTION[${n}]}" == "${section}" ]] && count=$((count + 1))
        done
        [[ "${count}" -gt 0 ]] || continue

        printf '\n## %s (%s)\n\n' "${section}" "${count}"
        printf '| Skill | Trigger |\n'
        printf '|-------|---------|\n'
        for n in "${sorted_names[@]}"; do
            [[ "${ROW_SECTION[${n}]}" == "${section}" ]] || continue
            printf '| `%s` | %s |\n' "${n}" "${DESC_OF[${n}]}"
        done
    done

    # Preserved verbatim from the hand-written catalog: judgement, not counts, so
    # it cannot drift from the filesystem.
    printf '\n---\n\n'
    cat <<'TRAILER'
## Consolidation Candidates

These pairs overlap and could merge if skill count becomes unwieldy:
- `memory-lifecycle-verification` + `turso-memory-verification` → `persistence-verification`
- `self-fix-loop` + `iterative-refinement` → `fix-loop`
- `goap-orchestrator` absorbs `task-decomposition` (already does wave decomposition)
TRAILER
}

# Render before touching the destination: a generator that died mid-write would
# otherwise leave a half-written catalog for the drift gate to compare against.
tmp_out="$(mktemp "${TMPDIR:-/tmp}/gen-skill-catalog.XXXXXX")"
trap 'rm -f "${tmp_out}"' EXIT
render_catalog > "${tmp_out}"

mkdir -p "$(dirname "${OUT_FILE}")"
cat -- "${tmp_out}" > "${OUT_FILE}"

printf 'wrote %s: %s skills, %s lines\n' \
    "${OUT_FILE#"${ROOT}/"}" "${#sorted_names[@]}" "$(wc -l < "${OUT_FILE}")"
