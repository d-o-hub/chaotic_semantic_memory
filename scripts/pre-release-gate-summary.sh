#!/usr/bin/env bash
# scripts/pre-release-gate-summary.sh — issue #903: compute the pre-release
# gate verdict from the seven needs results instead of inline workflow shell.
#
# WHY: workflow_run has no conclusion filter. Every gate job requires the
# triggering CI conclusion to be success, so a failed/cancelled CI run skips
# all seven checks. The old summary treated those skipped results as a gate
# failure, even though no gate check ran. On a successful CI run, skipped
# checks remain failures: a skipped need is not a pass.
#
# Usage: pre-release-gate-summary.sh <summary-file> <trigger-conclusion> <head-sha> <name=result>...
# Exit: 0 pass or no verdict, 1 a gate check failed, 2 usage/input error.
# Fixture: scripts/test-pre-release-gate-summary.sh

set -euo pipefail

usage() {
    printf 'Usage: %s <summary-file> <trigger-conclusion> <head-sha> <name=result>...\n' \
        "$(basename "$0")" >&2
    exit 2
}

if [[ "$#" -lt 4 ]]; then
    usage
fi

SUMMARY_FILE="$1"
TRIGGER_CONCLUSION="$2"
HEAD_SHA="$3"
shift 3

NAMES=()
RESULTS=()
for pair in "$@"; do
    if [[ "${pair}" != *=* || -z "${pair%%=*}" || -z "${pair#*=}" ]]; then
        printf "Error: expected <name=result>, got '%s'\n" "${pair}" >&2
        exit 2
    fi
    NAMES+=("${pair%%=*}")
    RESULTS+=("${pair#*=}")
done

case "${TRIGGER_CONCLUSION}" in
    success)
        VERDICT_MODE=true
        ;;
    failure|cancelled|timed_out|skipped|stale|action_required|startup_failure|neutral)
        VERDICT_MODE=false
        ;;
    *)
        printf "Error: unexpected workflow_run conclusion '%s' — the gate cannot compute a verdict from it\n" \
            "${TRIGGER_CONCLUSION}" >&2
        exit 2
        ;;
esac

FAILED_CHECKS=()
if [[ "${VERDICT_MODE}" == true ]]; then
    for index in "${!NAMES[@]}"; do
        case "${RESULTS[${index}]}" in
            success)
                ;;
            failure|cancelled|skipped)
                FAILED_CHECKS+=("${NAMES[${index}]}")
                ;;
            *)
                printf "Error: unexpected result '%s' for '%s' — expected success|failure|cancelled|skipped\n" \
                    "${RESULTS[${index}]}" "${NAMES[${index}]}" >&2
                exit 2
                ;;
        esac
    done
fi

{
    printf '%s\n\n' '## Pre-Release Gate Summary'
    printf '%s\n' '| Check | Status |' '|-------|--------|'
    for index in "${!NAMES[@]}"; do
        printf '| %s | %s |\n' "${NAMES[${index}]}" "${RESULTS[${index}]}"
    done
} >> "${SUMMARY_FILE}"

if [[ "${VERDICT_MODE}" == false ]]; then
    printf '\n%s\n' \
        "⏭️ **No verdict** — CI concluded \`${TRIGGER_CONCLUSION}\` for \`${HEAD_SHA}\`; every gate check was skipped by design. This run is not a pre-release gate verdict." \
        >> "${SUMMARY_FILE}"
    printf "::warning title=Pre-release gate::No verdict: CI concluded '%s' for %s; all gate checks were skipped by design.\n" \
        "${TRIGGER_CONCLUSION}" "${HEAD_SHA}"
    exit 0
fi

if [[ "${#FAILED_CHECKS[@]}" -eq 0 ]]; then
    printf '\n%s\n' '✅ **All blocking gates passed**' >> "${SUMMARY_FILE}"
    printf '%s\n' '✅ All blocking gates passed'
    exit 0
fi

join_names() {
    local separator=""
    local name
    for name in "$@"; do
        printf '%s%s' "${separator}" "${name}"
        separator=', '
    done
}

FAILED_LIST="$(join_names "${FAILED_CHECKS[@]}")"
printf '\n%s\n' '❌ **Gate failed** - one or more checks did not pass' >> "${SUMMARY_FILE}"
printf 'Failed checks: %s\n' "${FAILED_LIST}" >> "${SUMMARY_FILE}"
printf '❌ Gate failed: %s\n' "${FAILED_LIST}"
exit 1
