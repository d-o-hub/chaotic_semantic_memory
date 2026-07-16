#!/usr/bin/env bash
# =============================================================================
# validate-skill-format.sh - Fail-closed SKILL.md frontmatter + hard constraints
# =============================================================================
# Usage: ./scripts/validate-skill-format.sh [--verbose]
#
# Discovers .agents/skills/*/SKILL.md and validates:
#   1. YAML frontmatter between --- delimiters
#   2. Required fields: name, description (non-empty)
#   3. name matches parent directory name
#   4. SKILL.md line count <= 250 (hard fail)
#   5. If skill has references/, reference/, or scripts/, relative paths
#      mentioned in SKILL.md that look like local files must exist
#      (best-effort; URLs and anchors skipped)
#
# Exit codes:
#   0 - All SKILL.md files valid
#   1 - Format / constraint issues found
#   2 - Error occurred (usage / missing skills dir)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"
readonly MAX_SKILL_LOC=250

VERBOSE=false

for arg in "$@"; do
    case $arg in
        --verbose|-v) VERBOSE=true ;;
        --help|-h)
            cat << 'EOF'
Usage: validate-skill-format.sh [--verbose]

Fail-closed validation of .agents/skills/*/SKILL.md

Checks:
  - YAML frontmatter (--- ... ---) with name + description
  - name matches parent directory
  - SKILL.md <= 250 lines
  - Local references under references/, reference/, scripts/ resolve

Options:
  --verbose    Show details for each skill
  --help       Show this help message

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
echo -e "${CYAN}  SKILL.md Format Validation (fail-closed)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

if [[ ! -d "${SKILLS_DIR}" ]]; then
    echo -e "${RED}✗ Skills directory not found: ${SKILLS_DIR}${NC}"
    exit 2
fi

# Counters
TOTAL=0
VALID=0
ISSUES=0
NO_FRONTMATTER=0
MISSING_NAME=0
MISSING_DESC=0
NAME_MISMATCH=0
INVALID_YAML=0
LOC_VIOLATIONS=0
BROKEN_REFS=0

# Extract frontmatter body between first and second --- lines (stdin/file)
extract_frontmatter() {
    local file="$1"
    awk '
        /^---[[:space:]]*$/ {
            if (seen == 0) { seen = 1; next }
            if (seen == 1) { exit }
        }
        seen == 1 { print }
    ' "${file}"
}

# Parse simple YAML key: value (single-line only). Prints value or empty.
yaml_field() {
    local frontmatter="$1"
    local key="$2"
    local line raw
    line="$(printf '%s\n' "${frontmatter}" | grep -E "^${key}:" | head -n 1 || true)"
    [[ -z "${line}" ]] && return 0
    raw="${line#"${key}:"}"
    # trim leading/trailing whitespace
    raw="${raw#"${raw%%[![:space:]]*}"}"
    raw="${raw%"${raw##*[![:space:]]}"}"
    # strip one layer of matching quotes
    if [[ "${raw}" =~ ^\"(.*)\"$ ]]; then
        raw="${BASH_REMATCH[1]}"
    elif [[ "${raw}" =~ ^\'(.*)\'$ ]]; then
        raw="${BASH_REMATCH[1]}"
    fi
    printf '%s' "${raw}"
}

# Normalize a candidate path token (drop args, anchors, trailing punctuation)
normalize_path_token() {
    local p="$1"
    # First whitespace-delimited token only (handles `scripts/foo.sh <ver>`)
    p="${p%%[[:space:]]*}"
    # Drop markdown anchors and trailing punctuation from prose
    p="${p%%#*}"
    p="${p//$'\r'/}"
    p="${p%,}"
    p="${p%.}"
    p="${p%;}"
    p="${p%:}"
    p="${p#\'}"
    p="${p%\'}"
    p="${p#\"}"
    p="${p%\"}"
    printf '%s' "${p}"
}

# True if path looks like a skill-local file path we should resolve.
# Only references/, reference/, scripts/ (skill or repo-root scripts/) with
# a file extension — not bare repo filenames like Cargo.toml / AGENTS.md.
is_local_relpath() {
    local p
    p="$(normalize_path_token "$1")"
    # Skip URLs, anchors, empty, pure package/version tags
    [[ -z "${p}" ]] && return 1
    [[ "${p}" =~ ^https?:// ]] && return 1
    [[ "${p}" =~ ^mailto: ]] && return 1
    [[ "${p}" =~ ^# ]] && return 1
    [[ "${p}" =~ ^@ ]] && return 1
    # Skill-local or ./skill-local dirs only
    if [[ "${p}" =~ ^(references?|scripts)/ ]] || [[ "${p}" =~ ^\./(references?|scripts)/ ]]; then
        if [[ "${p}" =~ \.(md|sh|sql|json|ya?ml|toml|txt)$ ]]; then
            return 0
        fi
    fi
    return 1
}

# Resolve path relative to skill_dir; also try project root for scripts/
path_exists_for_skill() {
    local skill_dir="$1"
    local rel
    rel="$(normalize_path_token "$2")"
    # strip optional leading ./
    rel="${rel#./}"

    if [[ -e "${skill_dir}/${rel}" ]]; then
        return 0
    fi
    # Repo-root scripts are commonly referenced from skills
    if [[ "${rel}" == scripts/* ]] && [[ -e "${PROJECT_ROOT}/${rel}" ]]; then
        return 0
    fi
    return 1
}

# Collect candidate local paths from SKILL.md (markdown links + backtick paths)
collect_local_paths() {
    local skill_file="$1"
    local -a found=()
    local link bt norm

    # Markdown links: ](path) — strip optional title
    # shellcheck disable=SC2016  # single-quoted regex is intentional (no expansion)
    while IFS= read -r link; do
        # Drop optional "title" after path
        link="${link%%[[:space:]]*\"*}"
        link="${link%%[[:space:]]*\'*}"
        norm="$(normalize_path_token "${link}")"
        if is_local_relpath "${norm}"; then
            found+=("${norm}")
        fi
    done < <(grep -oE '\]\([^)]+\)' "${skill_file}" 2>/dev/null | sed 's/^](//; s/)$//' || true)

    # Backtick paths that look like local skill refs
    # shellcheck disable=SC2016  # single-quoted regex is intentional (no expansion)
    while IFS= read -r bt; do
        norm="$(normalize_path_token "${bt}")"
        if is_local_relpath "${norm}"; then
            found+=("${norm}")
        fi
    done < <(grep -oE '`[^`]+`' "${skill_file}" 2>/dev/null | sed 's/^`//; s/`$//' || true)

    # Deduplicate
    if [[ ${#found[@]} -gt 0 ]]; then
        printf '%s\n' "${found[@]}" | sort -u
    fi
}

check_skill_local_refs() {
    local skill_file="$1"
    local skill_dir
    skill_dir="$(dirname "${skill_file}")"
    local skill_name
    skill_name="$(basename "${skill_dir}")"
    local broken=0
    local path
    local any_path=false

    # Always resolve candidate local paths from markdown, even when
    # references/ or scripts/ directories are missing (false-negative fix).
    while IFS= read -r path; do
        [[ -z "${path}" ]] && continue
        any_path=true
        if ! path_exists_for_skill "${skill_dir}" "${path}"; then
            echo -e "${RED}  ✗${NC} ${skill_name}: missing path \`${path}\`"
            ((broken++)) || true
            ((BROKEN_REFS++)) || true
        elif $VERBOSE; then
            echo -e "${GREEN}  ·${NC} ${skill_name}: path ok \`${path}\`"
        fi
    done < <(collect_local_paths "${skill_file}")

    if ! $any_path && $VERBOSE; then
        echo -e "${GREEN}  ·${NC} ${skill_name}: no local path candidates"
    fi

    return "${broken}"
}

echo -e "${CYAN}→ Discovering SKILL.md under ${SKILLS_DIR}...${NC}"
echo ""

SKILL_FILES=()
while IFS= read -r skill_path; do
    [[ -n "${skill_path}" ]] || continue
    SKILL_FILES+=("${skill_path}")
done < <(find "${SKILLS_DIR}" -mindepth 2 -maxdepth 2 -name "SKILL.md" -type f | LC_ALL=C sort)

if [[ ${#SKILL_FILES[@]} -eq 0 ]]; then
    echo -e "${YELLOW}⚠ No SKILL.md files found in ${SKILLS_DIR}${NC}"
    exit 1
fi

echo -e "Inventory: ${#SKILL_FILES[@]} skill(s) on disk"
echo ""

for skill_file in "${SKILL_FILES[@]}"; do
    skill_dir_name="$(basename "$(dirname "${skill_file}")")"
    ((TOTAL++)) || true
    skill_ok=true
    fail_reasons=()

    # --- Frontmatter present (allow trailing whitespace on ---, match extractor) ---
    first_line="$(head -n 1 "${skill_file}" || true)"
    if [[ ! "${first_line}" =~ ^---[[:space:]]*$ ]]; then
        echo -e "${RED}✗${NC} ${skill_dir_name}: missing frontmatter (no --- at start)"
        ((NO_FRONTMATTER++)) || true
        ((ISSUES++)) || true
        continue
    fi

    frontmatter="$(extract_frontmatter "${skill_file}")"
    if [[ -z "${frontmatter}" ]]; then
        echo -e "${RED}✗${NC} ${skill_dir_name}: invalid frontmatter (missing closing ---)"
        ((INVALID_YAML++)) || true
        ((ISSUES++)) || true
        continue
    fi

    # Closing --- must exist (extract_frontmatter already requires second ---)
    # Ensure at least one non-empty field line
    if ! printf '%s\n' "${frontmatter}" | grep -qE '^[a-zA-Z_]'; then
        echo -e "${RED}✗${NC} ${skill_dir_name}: empty frontmatter"
        ((INVALID_YAML++)) || true
        ((ISSUES++)) || true
        continue
    fi

    skill_name="$(yaml_field "${frontmatter}" "name")"
    if [[ -z "${skill_name}" ]]; then
        fail_reasons+=("missing 'name' field")
        ((MISSING_NAME++)) || true
        skill_ok=false
    elif [[ "${skill_name}" != "${skill_dir_name}" ]]; then
        fail_reasons+=("name mismatch (frontmatter: '${skill_name}')")
        ((NAME_MISMATCH++)) || true
        skill_ok=false
    fi

    skill_desc="$(yaml_field "${frontmatter}" "description")"
    if [[ -z "${skill_desc}" ]]; then
        fail_reasons+=("missing or empty 'description' field")
        ((MISSING_DESC++)) || true
        skill_ok=false
    fi

    # --- LOC gate ---
    line_count="$(wc -l < "${skill_file}" | tr -d ' ')"
    if [[ "${line_count}" -gt "${MAX_SKILL_LOC}" ]]; then
        fail_reasons+=("${line_count} lines (max ${MAX_SKILL_LOC})")
        ((LOC_VIOLATIONS++)) || true
        skill_ok=false
    fi

    # --- Local path resolution (when references/scripts present) ---
    if ! check_skill_local_refs "${skill_file}"; then
        skill_ok=false
        fail_reasons+=("broken local path reference(s)")
    fi

    if $skill_ok; then
        echo -e "${GREEN}✓${NC} ${skill_dir_name}: valid (${line_count} lines)"
        if $VERBOSE; then
            echo "       Name: ${skill_name}"
            echo "       Description: ${skill_desc}"
        fi
        ((VALID++)) || true
    else
        echo -e "${RED}✗${NC} ${skill_dir_name}: ${fail_reasons[*]}"
        ((ISSUES++)) || true
    fi
done

# Summary
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Inventory (disk):    ${TOTAL}"
echo "Valid:               ${VALID}"
echo "Issues (skills):     ${ISSUES}"
echo "Missing frontmatter: ${NO_FRONTMATTER}"
echo "Missing name:        ${MISSING_NAME}"
echo "Missing description: ${MISSING_DESC}"
echo "Name mismatch:       ${NAME_MISMATCH}"
echo "Invalid YAML:        ${INVALID_YAML}"
echo "LOC > ${MAX_SKILL_LOC}:           ${LOC_VIOLATIONS}"
echo "Broken local refs:   ${BROKEN_REFS}"
echo ""

if [[ "${ISSUES}" -gt 0 ]]; then
    echo -e "${RED}✗ Found ${ISSUES} skill(s) with format/constraint issue(s)${NC}"
    echo ""
    echo "Requirements:"
    echo "  ---"
    echo "  name: <must-match-directory>"
    echo "  description: When to use this skill (non-empty)"
    echo "  ---"
    echo "  SKILL.md must be <= ${MAX_SKILL_LOC} lines"
    echo "  Paths under references/, reference/, scripts/ must resolve"
    exit 1
fi

echo -e "${GREEN}✓ All ${TOTAL} SKILL.md files pass fail-closed validation${NC}"
exit 0
