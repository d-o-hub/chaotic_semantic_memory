#!/usr/bin/env bash
# scripts/check-audit-ignore-parity.sh — issue #903: keep cargo-deny's reviewed
# RUSTSEC ignore set identical to the one cargo-audit actually reads.
#
# WHY: deny.toml and cargo-audit configuration are separate readers of the same
# security policy. A hand-maintained copy can drift, making the pre-release
# audit fail on reviewed advisories or silently omit a deny.toml exception.
# This checker compares IDs only inside each [advisories] section and fails
# closed if either file is absent, unreadable, or contains no parsed IDs.
#
# Usage: check-audit-ignore-parity.sh [deny-toml] [audit-toml]
# Defaults resolve from the repository root, independent of the caller's CWD.
# Exit: 0 sets match, 1 missing/invalid input or mismatch, 2 usage error.
# Fixture: scripts/test-check-audit-ignore-parity.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
    printf 'Usage: %s [deny-toml] [audit-toml]\n' "$(basename "$0")" >&2
    exit 2
}

if [[ "$#" -gt 2 ]]; then
    usage
fi

DENY_FILE="${1:-${ROOT}/deny.toml}"
AUDIT_FILE="${2:-${ROOT}/.cargo/audit.toml}"

no_ids_error() {
    printf 'Error: no RUSTSEC ids parsed from %s — refusing to read that as parity\n' "$1" >&2
    exit 1
}

extract_ids() {
    # Scan every [advisories] block: the negative control appends a second block,
    # and its ID must not be hidden by sed's single-range behavior.
    awk '
        /^\[advisories\][[:space:]]*$/ { in_advisories = 1; next }
        /^\[/ { in_advisories = 0 }
        in_advisories { print }
    ' "$1" |
        sed 's/#.*//' |
        grep -oE 'RUSTSEC-[0-9]{4}-[0-9]{4}' |
        sort -u
}

read_ids() {
    local file="$1"
    local ids

    if [[ ! -r "${file}" ]]; then
        no_ids_error "${file}"
    fi
    ids="$(extract_ids "${file}" || true)"
    if [[ -z "${ids}" ]]; then
        no_ids_error "${file}"
    fi
    printf '%s' "${ids}"
}

DENY_IDS="$(read_ids "${DENY_FILE}")"
AUDIT_IDS="$(read_ids "${AUDIT_FILE}")"

report_missing_ids() {
    local file="$1"
    local other="$2"
    local file_ids="$3"
    local other_ids="$4"
    local id

    while IFS= read -r id; do
        [[ -n "${id}" ]] || continue
        printf 'Error: %s lists %s but %s does not — add the reviewed ignore to both (issue #903).\n' \
            "${file}" "${id}" "${other}" >&2
    done < <(comm -23 <(printf '%s\n' "${file_ids}") <(printf '%s\n' "${other_ids}"))
}

DENY_ONLY="$(comm -23 <(printf '%s\n' "${DENY_IDS}") <(printf '%s\n' "${AUDIT_IDS}"))"
AUDIT_ONLY="$(comm -23 <(printf '%s\n' "${AUDIT_IDS}") <(printf '%s\n' "${DENY_IDS}"))"
if [[ -n "${DENY_ONLY}" || -n "${AUDIT_ONLY}" ]]; then
    report_missing_ids "${DENY_FILE}" "${AUDIT_FILE}" "${DENY_IDS}" "${AUDIT_IDS}"
    report_missing_ids "${AUDIT_FILE}" "${DENY_FILE}" "${AUDIT_IDS}" "${DENY_IDS}"
    exit 1
fi

COUNT="$(printf '%s\n' "${DENY_IDS}" | wc -l)"
COUNT="${COUNT//[[:space:]]/}"
printf 'ok: RUSTSEC ignore sets match (%s ids)\n' "${COUNT}"
