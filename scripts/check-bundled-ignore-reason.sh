#!/usr/bin/env bash
# scripts/check-bundled-ignore-reason.sh — issue #966.
#
# The zip advisory ignores in deny.toml / .cargo/audit.toml are only defensible
# while two independent facts stay true:
#   1. a `zip` package is still in Cargo.lock (otherwise the ignore is stale), and
#   2. `crates/csm-duckdb/Cargo.toml` still enables `duckdb/bundled` (otherwise
#      libduckdb-sys compiles the vulnerable network-archive extraction path).
#
# A `cargo deny` pass cannot see either fact: the RustSec database had no id for
# GHSA-m665-v7pm-7grj at filing time, and Cargo.lock cannot express dependency
# kinds. The same file also records the GHSA decision, so the next reader finds a
# measurement instead of re-deriving it.
#
# Usage: check-bundled-ignore-reason.sh [repo-root]
# Exit: 0 policy holds, 1 policy violated, 2 input could not be read (not a verdict).
# Fixture: scripts/test-check-bundled-ignore-reason.sh

set -euo pipefail

ROOT="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

DENY="${ROOT}/deny.toml"
AUDIT="${ROOT}/.cargo/audit.toml"
LOCK="${ROOT}/Cargo.lock"
DUCKDB="${ROOT}/crates/csm-duckdb/Cargo.toml"
ZIP_IDS=(RUSTSEC-2026-0367 RUSTSEC-2026-0368 RUSTSEC-2026-0369)
GHSA="GHSA-m665-v7pm-7grj"

fail() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

unreadable() {
    printf 'error: %s — the reason check cannot run\n' "$1" >&2
    exit 2
}

for file in "${DENY}" "${AUDIT}" "${LOCK}" "${DUCKDB}"; do
    [[ -r "${file}" ]] || unreadable "missing or unreadable ${file}"
done

ignored_ids=""
for file in "${DENY}" "${AUDIT}"; do
    while IFS= read -r id; do
        [[ -n "${id}" ]] && ignored_ids+="${id}"$'\n'
    done < <(awk '
        /^\[advisories\][[:space:]]*$/ { in_advisories = 1; next }
        /^\[/ { in_advisories = 0 }
        in_advisories { print }
    ' "${file}" | sed 's/#.*//' | grep -oE 'RUSTSEC-[0-9]{4}-[0-9]{4}' | sort -u)
done
ignored_ids="$(printf '%s' "${ignored_ids}" | sort -u)"
zip_ignored=0
for id in "${ZIP_IDS[@]}"; do
    if grep -qxF "${id}" <<< "${ignored_ids}"; then
        zip_ignored=1
    fi
done

zip_present=0
if grep -qE '^name = "zip"$' "${LOCK}"; then
    zip_present=1
fi

bundled=0
if grep -Eq 'duckdb[[:space:]]*=[[:space:]]*\{[^}]*features[[:space:]]*=[[:space:]]*\[[^]]*"bundled"' "${DUCKDB}"; then
    bundled=1
elif awk '
    /^\[dependencies\.duckdb\][[:space:]]*$/ { in_duckdb = 1; next }
    /^\[/ { in_duckdb = 0 }
    in_duckdb && /features[[:space:]]*=/ && /"bundled"/ { found = 1 }
    END { exit(found ? 0 : 1) }
' "${DUCKDB}"; then
    bundled=1
fi

# The record is required in every state: with the ignore (while zip is present)
# and after it is removed (so the next reader knows why it disappeared).
if ! grep -qF "${GHSA}" "${DENY}"; then
    fail "deny.toml does not record ${GHSA}; the Dependabot alert has no owner or re-check trigger"
fi
if ! grep -qiE 're-check' "${DENY}"; then
    fail "deny.toml records ${GHSA} without a re-check trigger"
fi

if [[ "${zip_ignored}" -eq 1 ]]; then
    if [[ "${zip_present}" -eq 0 ]]; then
        fail "a zip advisory is ignored but Cargo.lock has no zip package — the ignore outlived its reason"
    fi
    if [[ "${bundled}" -eq 0 ]]; then
        fail "a zip advisory is ignored but csm-duckdb does not enable duckdb/bundled — the vulnerable extraction path is compiled"
    fi
    printf 'ok: zip advisories ignored only while zip is locked and csm-duckdb is bundled (%s recorded)\n' "${GHSA}"
    exit 0
fi

if [[ "${zip_present}" -eq 1 ]]; then
    printf 'ok: zip is locked but no advisory is ignored; cargo deny owns the decision\n'
    exit 0
fi

printf 'ok: no zip package and no zip advisory ignore (%s recorded with a re-check trigger)\n' "${GHSA}"
