#!/usr/bin/env bash
# =============================================================================
# skill-eval.sh - Deterministic static evaluation of critical skills
# =============================================================================
# Usage: ./scripts/skill-eval.sh
#
# Scores the five critical skills against deterministic checkpoint tests
# derived from each skill's own SKILL.md contract (the assertions below are
# exact phrases/files the skill documents). Checks are purely static (grep
# and file existence) so the score is reproducible in CI without running
# cargo, git, or any network tool.
#
# Scoring:
#   - Each skill has 5 checkpoints x 4 assertions = 20 points.
#   - An assertion scores 1 point when its pattern is found in the skill's
#     SKILL.md body, or when the referenced file exists (file:), or when
#     the pattern is found inside a referenced file (head:).
#   - Skill pass threshold: >= 19/20. The eval exits non-zero as soon as
#     any critical skill scores below the threshold.
#
# Assertion kinds (lines are 'kind;target[;pattern]'):
#   text;<pattern>             grep -E '<pattern>' against <skill>/SKILL.md
#   file;<relpath>             <skill>/<relpath> must exist
#   head;<relpath>;<pattern>   grep -E '<pattern>' inside <skill>/<relpath>
#
# CI integration: run as `bash scripts/skill-eval.sh` in a job (see the
# skill-validation job in .github/workflows/ci.yml and validate.sh).
#
# Exit codes:
#   0 - every critical skill scored >= 19/20
#   1 - some critical skill scored below the threshold
#   2 - error occurred (missing skills dir, unknown skill)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SKILLS_DIR="${PROJECT_ROOT}/.agents/skills"
PASS_THRESHOLD=19
POINTS_PER_SKILL=20

CRITICAL_SKILLS=(
    git-workflow
    goap-planning
    rust-development
    testing-validation
    release-management
)

# Colors (portable across Linux/macOS)
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

# assertions_skill <skill> - prints each assertion as 'kind;target[;pattern]'
assertions_for() {
    local skill="$1"
    case "${skill}" in
        git-workflow)
            cat << 'EOF'
text;Conventional Commits
text;<type>\(<scope>\): <short summary in imperative mood>
head;references/commit-types.md;^# Commit Types and Scopes
head;references/commit-types.md;^\| `feat` \| New feature
text;references/commit-types.md
text;scripts/validate.sh
text;cargo clippy -- -D warnings
text;cargo fmt --check
text;cargo test --all-features --quiet
text;--save-baseline
text;reservoir_step_50k < 100μs
text;cargo bench --bench benchmark
text;--baseline main
text;gh pr checks
text;gh run list
text;pr-triage.sh
text;Do not claim success
text;--auto
text;merge one at a time
text;--force-with-lease
EOF
            ;;
        goap-planning)
            cat <<'EOF'
text;GOAP_STATE.md
text;measurable target state
text;Load state
text;current state
text;action-model.md
file;references/action-model.md
head;references/action-model.md;Preconditions: state facts that must already hold
head;references/action-model.md;magic-number tunables
text;planner-pattern.md
file;references/planner-pattern.md
head;references/planner-pattern.md;from heapq import heappop
head;references/planner-pattern.md;action.cost
text;Persist next action
text;update state after execution
text;minimal path
text;ordered, executable
text;preconditions
text;effects
text;costs
head;references/action-model.md;Cost: relative effort
EOF
            ;;
        rust-development)
            cat <<'EOF'
text;AGENTS.md
text;codebase-patterns.md
file;reference/codebase-patterns.md
text;scripts/validate.sh
text;must be ≤ 500 lines
text;LOC gate applies to `crates/` too
text;proactively split
text;crates/\*/src/\*.rs
text;Result<T, MemoryError>
text;Tokio async for I/O
text;libsql only
text;turso-client
text;\[0.9, 1.1\]
text;StdRng
text;SeedableRng
text;No magic numbers
text;pub mod name;
text;prelude
text;Module Map
text;src/persistence.rs
EOF
            ;;
        testing-validation)
            cat <<'EOF'
text;scripts/validate.sh
text;cargo check --message-format=short
text;cargo test --all-features --quiet
text;cargo clippy -- -D warnings
text;--save-baseline
text;--baseline main
text;fails silently
text;reservoir_step < 100μs
text;loc-check.sh
file;scripts/loc-check.sh
file;scripts/validate.sh
text;check-docs-links.sh
text;--fix
text;--check-urls
text;Version references consistency
text;Cargo.lock
text;NamedTempFile
text;new_seeded
text;Criterion closures
text;hardcoded tunables
EOF
            ;;
        release-management)
            cat <<'EOF'
text;release-manager.sh
text;single-owner
text;Never run `git tag`
text;explicit human approval
text;wait-for-ci
text;scripts/validate.sh
text;release-manager.sh prepare
text;release-manager.sh validate
text;gh release view v
text;## \[0\.2\.9\] - 2026-04-06
text;\[unreleased\]
text;SemVer
text;BREAKING CHANGE
text;dist-channel-selection
text;@d-o-hub/csm
text;Trusted Publisher
text;cargo yank
file;references/release-workflow.md
file;references/trusted-publishing.md
file;references/version-tag-format.md
EOF
            ;;
        *)
            echo "unknown-skill" >&2
            return 2
            ;;
    esac
}

# score_skill <skill> -> prints "score;misses" on stdout
score_skill() {
    local skill="$1"
    local skill_file="${SKILLS_DIR}/${skill}/SKILL.md"
    local score=0
    local misses=""
    local count=0

    if [[ ! -f "${skill_file}" ]]; then
        echo "0;SKILL.md missing: ${skill_file}"
        return 0
    fi

    local kind target pattern
    while IFS=';' read -r kind target pattern; do
        [[ -z "${kind}" ]] && continue
        count=$((count + 1))
        case "${kind}" in
            text)
                if grep -qE -- "${target}" "${skill_file}"; then
                    score=$((score + 1))
                else
                    misses="${misses} text:${target}"
                fi
                ;;
            file)
                if [[ -f "${SKILLS_DIR}/${skill}/${target}" ]]; then
                    score=$((score + 1))
                else
                    misses="${misses} file:${target}"
                fi
                ;;
            head)
                if grep -qE -- "${pattern}" "${SKILLS_DIR}/${skill}/${target}"; then
                    score=$((score + 1))
                else
                    misses="${misses} head:${target} =~ ${pattern}"
                fi
                ;;
            *)
                misses="${misses} unknown-kind:${kind}"
                ;;
        esac
    done <<< "$(assertions_for "${skill}")"

    if [[ "${count}" -ne "${POINTS_PER_SKILL}" ]]; then
        misses="${misses} (assertion count ${count} != ${POINTS_PER_SKILL})"
    fi
    printf '%s;%s' "${score}" "${misses}"
}

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${CYAN}  Critical Skill Evaluation (static checkpoints, threshold ≥ ${PASS_THRESHOLD}/20)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

if [[ ! -d "${SKILLS_DIR}" ]]; then
    echo -e "${RED}✗ Skills directory not found: ${SKILLS_DIR}${NC}"
    exit 2
fi

OVERALL=0
FAILED_SKILLS=""

printf "%-24s %8s %s\n" "Skill" "Score" "Status"
for skill in "${CRITICAL_SKILLS[@]}"; do
    result=$(score_skill "${skill}")
    score="${result%%;*}"
    misses="${result#*;}"
    if [[ -n "${misses}" ]]; then
        misses="${misses# }"
    fi
    if [[ "${score}" -ge "${PASS_THRESHOLD}" ]]; then
        printf "%-24s %4s/%-4s %s\n" "${skill}" "${score}" "${POINTS_PER_SKILL}" "PASS"
    else
        printf "%-24s %4s/%-4s %s\n" "${skill}" "${score}" "${POINTS_PER_SKILL}" "FAIL"
        OVERALL=$((OVERALL + 1))
        FAILED_SKILLS="${FAILED_SKILLS}\n  - ${skill} (${score}/${POINTS_PER_SKILL})${misses}"
    fi
done

echo ""
if [[ "${OVERALL}" -gt 0 ]]; then
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${RED}✗ ${OVERALL} critical skill(s) scored below ${PASS_THRESHOLD}/20${NC}"
    echo -e "${RED}Missed assertions:${NC}"
    echo -e "${FAILED_SKILLS}"
    exit 1
else
    echo -e "${GREEN}✓ All ${#CRITICAL_SKILLS[@]} critical skills scored ≥ ${PASS_THRESHOLD}/20${NC}"
    exit 0
fi