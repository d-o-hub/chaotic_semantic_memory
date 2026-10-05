#!/usr/bin/env bash
# =============================================================================
# ci-workspace-crate-matrix.sh — derive the CI `test-workspace-crates` matrix
# from the real workspace (`cargo metadata`) instead of a hand-written list.
# =============================================================================
# WHY THIS GATE EXISTS (issue #827, audit A6 remainder)
# The matrix was eight hand-written `- csm-*` entries guarded only by a
# "Keep in sync with workspace members" comment. A comment is not a gate: a new
# `crates/csm-<new>` member compiles, publishes and ships while
# `cargo test -p csm-<new>` never runs, and nothing reports the omission. This
# script reads the authoritative workspace graph, so a new member is IN CI by
# default and the duplicated list is gone.
#
# WHAT IT ENFORCES (any violation is exit 1)
#   1. Membership   — every `crates/<dir>` workspace package is a matrix
#      candidate. The root crate (`.`) and `benchmarks` are not: they have
#      dedicated jobs (`test`, `test-benchmarks`).
#   2. Exclusions   — a candidate may only leave the matrix when a *dedicated
#      job* provably covers it. Each EXCLUSIONS entry must (a) still be a
#      `crates/*` workspace member and (b) appear in the workflow as a
#      non-comment `-p <crate>` argument. An exclusion whose job was deleted is
#      an error, so the list cannot rot back into a silent skip.
#   3. Non-emptiness — an empty derived matrix is an ERROR. `matrix: {crate: []}`
#      makes Actions run zero jobs and report success, so a failed/garbled
#      metadata read would otherwise reproduce exactly the bug this gate fixes.
#   4. No list re-introduced — if literal `- csm-*` entries reappear under
#      `test-workspace-crates`' `strategy.matrix`, or the matrix stops using
#      `fromJSON(...)`, that is an error. The point was to delete the duplicate
#      source of truth, not to keep a second one next to the derived one.
#
# BLIND SPOTS (stated so nobody trusts this gate more than it deserves)
#   - It is textual, not YAML-parsed. The workflow is sliced with awk at 2-space
#     job keys and grepped for `-p <crate>`, so a crate covered by
#     `cargo test --workspace`, or by `-p $SOME_VAR`, is NOT recognised as
#     covered. That fails in the safe direction (the exclusion is rejected).
#   - It proves the argument appears, not that the job RUNS: a path-filtered
#     `if:` job, an `if: false` job or a `|| true` step still counts as coverage.
#     An exclusion therefore still needs a human to own it — but now against a
#     named, greppable anchor instead of a prose comment.
#   - Only `crates/*` members are candidates. A member added elsewhere (like
#     `benchmarks`) never reaches this matrix and needs its own job.
#   - `--locked` and the feature set of the per-crate test step are unchanged
#     from the hand-written job; this script only decides WHICH crates run.
#
# Usage (paths resolve relative to this script, so it runs from anywhere):
#   scripts/ci-workspace-crate-matrix.sh                  # names, one per line
#   scripts/ci-workspace-crate-matrix.sh --json           # {"crate":["a","b"]} one line
#   scripts/ci-workspace-crate-matrix.sh --github-output  # append `crates=<json>` to $GITHUB_OUTPUT
#   scripts/ci-workspace-crate-matrix.sh --check          # validate + summary, no list
#   scripts/ci-workspace-crate-matrix.sh --help
#
# Test hooks (used by scripts/test-ci-workspace-crate-matrix.sh; CI sets none of
# them, so the production path always reads real `cargo metadata` + real ci.yml):
#   CSM_CARGO_METADATA_JSON  read workspace metadata from this file, not cargo
#   CSM_CI_WORKFLOW          workflow whose dedicated jobs justify the exclusions
#   CSM_MATRIX_EXCLUSIONS    space-separated exclusion list override
#
# Negative fixture: scripts/test-ci-workspace-crate-matrix.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CI_WORKFLOW="${CSM_CI_WORKFLOW:-${REPO_ROOT}/.github/workflows/ci.yml}"
MATRIX_JOB="test-workspace-crates"

# `crates/*` members covered by a dedicated CI job instead of this matrix. The
# reason is a contract, not prose: the job has to exist (enforcement 2b).
DEFAULT_EXCLUSIONS="csm-duckdb csm-wasm"
if [[ -n "${CSM_MATRIX_EXCLUSIONS:-}" ]]; then
    read -r -a EXCLUSIONS <<< "${CSM_MATRIX_EXCLUSIONS}"
else
    read -r -a EXCLUSIONS <<< "${DEFAULT_EXCLUSIONS}"
fi

show_help() {
    cat <<'HELP'
Usage: scripts/ci-workspace-crate-matrix.sh [--json|--github-output|--check|--help]

Derives the `test-workspace-crates` CI matrix from the `crates/*` workspace
members reported by `cargo metadata --no-deps`, minus crates that have a
dedicated job (verified against the workflow, not against a comment).

Modes:
  (no flag)        crate names, one per line (sorted, deterministic)
  --json           GitHub Actions matrix object: {"crate":["a","b"]} on one line
  --github-output  append `crates=<json>` to $GITHUB_OUTPUT (for the derive job)
  --check          validate the invariants and print a summary, emit no list
  --help           this message

Exit codes: 0 valid, 1 invariant violated, 2 bad flag.
HELP
}

MODE="list"
case "${1:-}" in
    "")                MODE="list" ;;
    --json)            MODE="json" ;;
    --github-output)   MODE="github-output" ;;
    --check)           MODE="check" ;;
    --help|-h)         show_help; exit 0 ;;
    *)
        echo "Error: unknown flag: $1 (see --help)" >&2
        exit 2 ;;
esac

fail() {
    printf 'ci crate matrix gate failed: %s\n' "$1" >&2
    shift
    while [ "$#" -gt 0 ]; do
        printf '  %s\n' "$1" >&2
        shift
    done
    exit 1
}

# ---------------------------------------------------------------------------
# 0. Dependencies and inputs.
# ---------------------------------------------------------------------------
command -v jq >/dev/null 2>&1 \
    || fail "jq is not installed; the workspace crate matrix cannot be derived without it"

if [[ -n "${CSM_CARGO_METADATA_JSON:-}" ]]; then
    [[ -f "${CSM_CARGO_METADATA_JSON}" ]] \
        || fail "metadata fixture not found: ${CSM_CARGO_METADATA_JSON}" \
            "CSM_CARGO_METADATA_JSON must point at cargo metadata --format-version 1 output"
    metadata="$(cat "${CSM_CARGO_METADATA_JSON}")"
else
    command -v cargo >/dev/null 2>&1 \
        || fail "cargo is not installed; the workspace crate matrix cannot be derived without it"
    # --no-deps: manifest-only, so no registry access and no lockfile resolution.
    if ! metadata="$(cd "${REPO_ROOT}" && cargo metadata --no-deps --format-version 1 2>&1)"; then
        fail "cargo metadata failed to describe the workspace" "${metadata}"
    fi
fi

if ! printf '%s' "${metadata}" | jq -e '.packages | type == "array"' >/dev/null 2>&1; then
    fail "workspace metadata has no .packages array" \
        "expected \`cargo metadata --no-deps --format-version 1\` output, got:" \
        "$(printf '%s' "${metadata}" | head -c 200)"
fi

workspace_root="$(printf '%s' "${metadata}" | jq -r '.workspace_root // ""')"

# Members whose manifest lives at <workspace_root>/crates/<dir>/Cargo.toml. The
# root crate and any out-of-tree member (e.g. `benchmarks`) never match.
all_crates=()
while IFS= read -r crate; do
    [[ -n "${crate}" ]] && all_crates+=("${crate}")
done < <(
    printf '%s' "${metadata}" \
        | jq -r --arg wr "${workspace_root}" '
            .packages[]
            | select(
                if $wr == "" then
                    (.manifest_path | test("/crates/[^/]+/Cargo.toml$"))
                else
                    (.manifest_path | startswith($wr + "/crates/"))
                end
              )
            | .name' \
        | sort -u
)

if [[ "${#all_crates[@]}" -eq 0 ]]; then
    fail "no crates/* workspace members found in the metadata" \
        "a metadata read that silently returns nothing would make every exclusion unprovable" \
        "workspace_root='${workspace_root}'"
fi

# ---------------------------------------------------------------------------
# 1. The workflow is both the consumer of this matrix and the evidence for
#    every exclusion, so it must be present.
# ---------------------------------------------------------------------------
[[ -f "${CI_WORKFLOW}" ]] \
    || fail "workflow not found: ${CI_WORKFLOW}" \
        "exclusions cannot be proven without the workflow that owns the jobs"

# Non-comment lines only: a prose mention of `-p csm-foo` must never count as
# coverage for csm-foo.
workflow_code="$(grep -v '^[[:space:]]*#' "${CI_WORKFLOW}" || true)"

# ---------------------------------------------------------------------------
# 2. Exclusions must be real members with a dedicated job (2a + 2b).
# ---------------------------------------------------------------------------
for crate in "${EXCLUSIONS[@]}"; do
    if ! printf '%s\n' "${all_crates[@]}" | grep -qxF "${crate}"; then
        fail "excluded crate is no longer a workspace member: ${crate}" \
            "drop it from DEFAULT_EXCLUSIONS in scripts/ci-workspace-crate-matrix.sh"
    fi
    if ! printf '%s\n' "${workflow_code}" \
        | grep -Eq -- "(^|[^[:alnum:]_-])-p[[:space:]]+${crate}([^[:alnum:]_-]|$)"; then
        fail "excluded crate has no dedicated job in $(basename "${CI_WORKFLOW}"): ${crate}" \
            "either give it a '-p ${crate}' step in a job, or drop it from DEFAULT_EXCLUSIONS so the matrix tests it"
    fi
done

# ---------------------------------------------------------------------------
# 3. Matrix = candidates - exclusions, and it must be non-empty.
# ---------------------------------------------------------------------------
matrix_crates=()
for crate in "${all_crates[@]}"; do
    skip="false"
    for excluded in "${EXCLUSIONS[@]}"; do
        [[ "${crate}" == "${excluded}" ]] && skip="true"
    done
    [[ "${skip}" == "true" ]] && continue
    matrix_crates+=("${crate}")
done

if [[ "${#matrix_crates[@]}" -eq 0 ]]; then
    fail "the derived matrix is empty" \
        "GitHub Actions runs zero jobs for an empty matrix and reports success — the silent skip this gate exists to prevent" \
        "crates/* members seen: ${all_crates[*]}" \
        "exclusions applied: ${EXCLUSIONS[*]}"
fi

# ---------------------------------------------------------------------------
# 4. The workflow must consume the derived matrix, not a re-typed list.
#    Job block = `  test-workspace-crates:` up to the next 2-space job key.
# ---------------------------------------------------------------------------
job_block="$(awk -v job="${MATRIX_JOB}:" '
    $0 ~ "^  " job { inblock = 1; print; next }
    inblock && $0 ~ /^  [A-Za-z0-9_][A-Za-z0-9_-]*:/ { inblock = 0 }
    inblock { print }
' "${CI_WORKFLOW}")"

if [[ -z "${job_block}" ]]; then
    fail "job '${MATRIX_JOB}' not found in $(basename "${CI_WORKFLOW}")" \
        "the derived matrix has no consumer; restore the job or update MATRIX_JOB here"
fi

if ! printf '%s\n' "${job_block}" | grep -Eq 'matrix:[[:space:]]*\$\{\{[[:space:]]*fromJSON\('; then
    fail "job '${MATRIX_JOB}' no longer derives its matrix from this script" \
        "expected a line such as: matrix: \${{ fromJSON(needs.workspace-matrix.outputs.crates) }}"
fi

literal_entries=()
while IFS= read -r entry; do
    [[ -n "${entry}" ]] && literal_entries+=("${entry}")
done < <(printf '%s\n' "${job_block}" \
    | grep -E '^[[:space:]]+-[[:space:]]+(csm|chaotic)[[:alnum:]_-]+' || true)

if [[ "${#literal_entries[@]}" -gt 0 ]]; then
    fail "hand-written crate entries are back in the '${MATRIX_JOB}' matrix" \
        "delete them — this script is the only list, and a second copy diverges invisibly:" \
        "${literal_entries[@]}"
fi

# ---------------------------------------------------------------------------
# 5. Emit in the requested mode.
# ---------------------------------------------------------------------------
json="$(printf '%s\n' "${matrix_crates[@]}" \
    | jq -R -s -c '{crate: (split("\n") | map(select(length > 0)))}')"

# The payload is what `fromJSON()` receives, so verify its shape here rather
# than letting Actions fail on an unparsable output value.
if ! printf '%s' "${json}" | jq -e '.crate | (type == "array") and (length > 0)' >/dev/null 2>&1; then
    fail "the emitted matrix JSON is not a non-empty {crate:[...]} object" "got: ${json}"
fi

case "${MODE}" in
    list)
        printf '%s\n' "${matrix_crates[@]}"
        ;;
    json)
        printf '%s\n' "${json}"
        ;;
    github-output)
        [[ -n "${GITHUB_OUTPUT:-}" ]] \
            || fail "--github-output needs GITHUB_OUTPUT (every Actions step exports it)"
        # One `key=value` line: a value containing a newline would silently become
        # a multi-line output, so assert the single-line shape instead of relying
        # on fromJSON() tolerating trailing whitespace.
        if [[ "${json}" == *$'\n'* ]]; then
            fail "the emitted matrix JSON is not single-line; refusing to write it to GITHUB_OUTPUT" \
                "got: ${json}"
        fi
        printf 'crates=%s\n' "${json}" >> "${GITHUB_OUTPUT}"
        printf 'wrote matrix for %s to GITHUB_OUTPUT: %s\n' "${MATRIX_JOB}" "${json}"
        ;;
    check)
        printf 'ok: %s crate(s) derived for the %s matrix: %s\n' \
            "${#matrix_crates[@]}" "${MATRIX_JOB}" "${matrix_crates[*]}"
        printf 'ok: %s crate(s) excluded because a dedicated job covers them: %s\n' \
            "${#EXCLUSIONS[@]}" "${EXCLUSIONS[*]:-none}"
        printf 'ok: %s consumes the derived matrix and holds no hand-written entries\n' "${MATRIX_JOB}"
        ;;
esac
