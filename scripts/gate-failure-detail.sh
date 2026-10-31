#!/usr/bin/env bash
# scripts/gate-failure-detail.sh — issue #960: turn "Pre-Release Gate is
# completed/failure" into the name of the job that failed.
#
# WHY: release.yml's gate step prints only the blended run verdict, so a RUSTSEC
# finding, a red benchmark and "somebody has an open PR" are the same nine
# characters of output at the point of refusal. The per-job conclusions already
# exist one hop away (GET /actions/runs/{id}/jobs), and gate-summary prints them
# ("PR Drain=failure", pre-release-gate.yml:197) — but that is a different
# workflow's summary, not the release log an operator reads. Surfacing the name
# strengthens the queue policy instead of weakening it: the correct remedy
# (resolve or label the PR) becomes visible exactly where the release stopped.
#
# The jobs JSON is read from stdin so the parsing is deterministic and needs no
# network access in the fixture: scripts/test-gate-failure-detail.sh.
#
# Usage: gh api repos/<repo>/actions/runs/<id>/jobs | \
#          gate-failure-detail.sh <run-id> <head-sha>
# Output: one "  <job>: <status>/<conclusion>" line per job, then one
#         "SUMMARY: …" single-line digest intended for a ::error:: annotation.
# Exit:   0 always for a readable payload — a diagnostic must never become the
#         reason a release refuses; the caller already holds that verdict.
#         2 only on argument misuse (a bug in the caller).

set -uo pipefail

usage() {
    printf 'Usage: %s <run-id> <head-sha>  (job list JSON on stdin)\n' "$(basename "$0")" >&2
    exit 2
}

if [[ "$#" -ne 2 ]]; then
    usage
fi

RUN_ID="$1"
HEAD_SHA="$2"

# Job names are workflow strings; collapse newlines and :: so a payload cannot
# forge an extra annotation line through this script's stdout.
sanitize() {
    printf '%s' "$1" | tr -d '\r\n' | sed 's/::/:/g'
}

PAYLOAD="$(cat)"

if ! printf '%s' "${PAYLOAD}" | jq -e 'has("jobs")' >/dev/null 2>&1; then
    printf 'SUMMARY: gate job detail unavailable for run %s on %s (no jobs array)\n' \
        "$(sanitize "${RUN_ID}")" "$(sanitize "${HEAD_SHA}")"
    exit 0
fi

TOTAL="$(printf '%s' "${PAYLOAD}" | jq -r '.total_count // (.jobs | length)')"
if [[ "${TOTAL}" -eq 0 ]]; then
    printf 'SUMMARY: gate run %s has ZERO jobs — GitHub rejected the workflow file before scheduling, so there is no step log to read (jobless-run rule: .agents/skills/release-management/SKILL.md hard rule 4; scripts/check-actionlint.sh is the gate)\n' \
        "$(sanitize "${RUN_ID}")"
    exit 0
fi

FAILED=()
NO_VERDICT=()
SKIPPED=()
SUCCEEDED=0

while IFS='|' read -r job_name job_status job_conclusion; do
    job_name="$(sanitize "${job_name}")"
    if [[ "${job_status}" != "completed" ]]; then
        NO_VERDICT+=("${job_name}(${job_status})")
        printf '  %s: %s/%s\n' "${job_name}" "${job_status}" "${job_conclusion}"
        continue
    fi
    case "${job_conclusion}" in
        success)
            SUCCEEDED=$((SUCCEEDED + 1))
            ;;
        skipped)
            SKIPPED+=("${job_name}")
            ;;
        *)
            FAILED+=("${job_name}")
            ;;
    esac
    printf '  %s: %s/%s\n' "${job_name}" "${job_status}" "${job_conclusion}"
done < <(printf '%s' "${PAYLOAD}" | jq -r \
    '.jobs[] | "\(.name | gsub("[\r\n]"; " "))|\(.status)|\(.conclusion // "none")"')

join_names() {
    local separator=""
    local name
    for name in "$@"; do
        printf '%s%s' "${separator}" "${name}"
        separator=', '
    done
}

FAILED_LIST="$(join_names "${FAILED[@]+"${FAILED[@]}"}")"
NO_VERDICT_LIST="$(join_names "${NO_VERDICT[@]+"${NO_VERDICT[@]}"}")"
SKIPPED_LIST="$(join_names "${SKIPPED[@]+"${SKIPPED[@]}"}")"

# The refusal case with no explanation is the one #960 was filed for; an
# all-green job list under a failed run verdict is the #914 class (skipped
# needs read as a failure) and says so rather than contradicting itself.
if [[ "${#FAILED[@]}" -gt 0 ]]; then
    printf 'SUMMARY: %s of %s gate jobs failed: %s' \
        "${#FAILED[@]}" "${TOTAL}" "${FAILED_LIST}"
elif [[ "${#NO_VERDICT[@]}" -gt 0 ]]; then
    printf 'SUMMARY: %s gate job(s) had no verdict yet: %s' \
        "${#NO_VERDICT[@]}" "${NO_VERDICT_LIST}"
elif [[ "${#SKIPPED[@]}" -gt 0 ]]; then
    printf 'SUMMARY: no failed job; %s skipped: %s (a skipped need is not a pass — #914 class)' \
        "${#SKIPPED[@]}" "${SKIPPED_LIST}"
else
    printf 'SUMMARY: all %s gate jobs completed/success; the run-level verdict disagrees with its jobs' \
        "${SUCCEEDED}"
fi
printf ' (run %s, %s)\n' "$(sanitize "${RUN_ID}")" "$(sanitize "${HEAD_SHA}")"
exit 0
