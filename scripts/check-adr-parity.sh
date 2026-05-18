#!/usr/bin/env bash
# scripts/check-adr-parity.sh
#
# ADR-0076: Enforce parity between plans/ADR_REGISTRY.md and on-disk ADR files
# in plans/adr/ and docs/adr/.
#
# Exits non-zero if any registry entry has no backing file (other than the
# documented superseded/N/A sentinels).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REGISTRY="${ROOT}/plans/ADR_REGISTRY.md"

if [[ ! -f "${REGISTRY}" ]]; then
    echo "error: registry not found at ${REGISTRY}" >&2
    exit 2
fi

# Extract every 4-digit ADR id from registry table rows (lines starting with "| 00").
registry_ids=$(grep -oE '^\| 00[0-9]{2}' "${REGISTRY}" | grep -oE '00[0-9]{2}' | sort -u)

# Collect IDs that the registry marks as Superseded / N/A — these are
# allowed to have no on-disk file.
superseded_ids=$(awk -F'|' '/^\| 00[0-9]{2}/ && /N\/A/ { gsub(/ /, "", $2); print $2 }' "${REGISTRY}" | sort -u)

# Extract on-disk ADR ids from both canonical locations.
disk_ids=$(find "${ROOT}/plans/adr" "${ROOT}/docs/adr" -maxdepth 1 -name '00*.md' -printf '%f\n' 2>/dev/null \
    | grep -oE '^00[0-9]{2}' \
    | sort -u)

# Registry IDs missing on disk and not flagged as superseded.
missing=$(comm -23 \
    <(printf '%s\n' "${registry_ids}") \
    <(printf '%s\n%s\n' "${disk_ids}" "${superseded_ids}" | sort -u))

if [[ -n "${missing}" ]]; then
    echo "error: registry references ADRs with no on-disk source of truth:" >&2
    while IFS= read -r id; do printf '  ADR-%s\n' "${id}" >&2; done <<< "${missing}"
    echo "" >&2
    echo "Fix: add the missing file under plans/adr/ or docs/adr/, or mark the" >&2
    echo "row in plans/ADR_REGISTRY.md as 'Superseded' with file 'N/A'." >&2
    exit 1
fi

# On-disk ADRs missing from registry (forward parity).
orphan=$(comm -23 \
    <(printf '%s\n' "${disk_ids}") \
    <(printf '%s\n' "${registry_ids}"))

if [[ -n "${orphan}" ]]; then
    echo "warning: ADR files present on disk but not in registry:" >&2
    while IFS= read -r id; do printf '  ADR-%s\n' "${id}" >&2; done <<< "${orphan}"
    # Warning only — don't fail the build for forward additions before the
    # author has had a chance to add the registry row.
fi

registry_count=$(printf '%s\n' "${registry_ids}" | wc -l | tr -d ' ')
disk_count=$(printf '%s\n' "${disk_ids}" | wc -l | tr -d ' ')
echo "ok: ADR parity satisfied (registry=${registry_count}, disk=${disk_count})"
