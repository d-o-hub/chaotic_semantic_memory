#!/usr/bin/env bash
# scripts/check-archive-manifest.sh
#
# Issue #831: `plans/ARCHIVE_MANIFEST.md` is the only index of `plans/.archive/`, and
# until now nothing read it. `scripts/plans-manager.sh archive adr` moves ADR files
# straight into the `plans/.archive/` root without touching the manifest, so the
# "complete" index silently went unlisted for 55 archived ADRs and 49 wave handoffs while
# the GOAP flag `plan_archive_manifest_valid` rested on prose. This makes that flag a gate.
#
# Bidirectional, measured against the tree:
#   * every file on disk under plans/.archive/ must be enumerated in the manifest
#     (unlisted file -> exit 1)
#   * every manifest row naming a file under plans/.archive/ must exist on disk
#     (stale row -> exit 1)
#
# Matching convention: the manifest enumerates each file by its repo-relative path in
# backticks (`` `plans/.archive/0001-use-libsql.md` ``). Any backticked path under
# plans/.archive/ whose last segment looks like a file counts as an enumeration row,
# wherever it sits — so the table layout, section grouping and the description column
# stay free-form, and a non-.md archived file is listable. Directory paths, globs and
# placeholders (`<name>`) are ignored. Unlisted files are reported paste-ready.
#
# Exits 2 if the manifest or plans/.archive/ is missing entirely (a broken tree, not a
# manifest drift).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="${ROOT}/plans/ARCHIVE_MANIFEST.md"
ARCHIVE_DIR="${ROOT}/plans/.archive"

if [[ ! -f "${MANIFEST}" ]]; then
    echo "error: manifest not found at ${MANIFEST}" >&2
    exit 2
fi

if [[ ! -d "${ARCHIVE_DIR}" ]]; then
    echo "error: archive directory not found at ${ARCHIVE_DIR}" >&2
    exit 2
fi

# Ground truth: every file under plans/.archive/, repo-relative, sorted.
mapfile -t disk_files < <(cd "${ROOT}" && find plans/.archive -type f | LC_ALL=C sort)

if [[ ${#disk_files[@]} -eq 0 ]]; then
    echo "error: no files found under ${ARCHIVE_DIR}" >&2
    exit 2
fi

# What the manifest enumerates: every backticked plans/.archive/ path whose last segment
# looks like a file (contains a dot, no trailing slash, no glob/placeholder characters).
# Any extension counts, so a non-.md file on disk can always be listed; directory paths
# (`plans/.archive/`), placeholders (`<name>`) and prose ellipses are not rows.
# shellcheck disable=SC2016  # the backticks in the pattern are the literal manifest
# delimiter, not a command substitution.
listed_raw=$(grep -oE '`plans/\.archive/[^`]*`' "${MANIFEST}" \
    | tr -d '`' \
    | grep -vE '/$|[<>*?]' \
    | grep -E '\.[^/]+$' \
    | LC_ALL=C sort -u || true)

declare -A listed_set=()
while IFS= read -r path; do
    if [[ -n "${path}" ]]; then
        listed_set["${path}"]=1
    fi
done <<< "${listed_raw}"

# Direction 1: on disk but not in the manifest.
missing=()
for path in "${disk_files[@]}"; do
    if [[ -z "${listed_set[${path}]:-}" ]]; then
        missing+=("${path}")
    fi
done

# Direction 2: in the manifest but not on disk (or not a regular file).
stale=()
if [[ ${#listed_set[@]} -gt 0 ]]; then
    mapfile -t listed_sorted < <(printf '%s\n' "${!listed_set[@]}" | LC_ALL=C sort)
    for path in "${listed_sorted[@]}"; do
        if [[ ! -f "${ROOT}/${path}" ]]; then
            stale+=("${path}")
        fi
    done
fi

if [[ ${#stale[@]} -gt 0 ]]; then
    echo "error: ${#stale[@]} manifest row(s) name files that do not exist:" >&2
    for path in "${stale[@]}"; do
        printf '  %s\n' "${path}" >&2
    done
    echo "" >&2
    echo "Fix: restore the file under plans/.archive/ (the archive policy is" >&2
    echo "non-destructive, ADR-0096) or delete the row if the file was genuinely" >&2
    echo "removed from the archive in a tracked commit." >&2
    exit 1
fi

if [[ ${#missing[@]} -gt 0 ]]; then
    echo "error: ${#missing[@]} file(s) exist under plans/.archive/ but are not" >&2
    echo "       enumerated in plans/ARCHIVE_MANIFEST.md:" >&2
    for path in "${missing[@]}"; do
        if grep -qF -- "$(basename "${path}")" "${MANIFEST}"; then
            printf '  %s  (basename appears in the manifest, but not as a' "${path}" >&2
            echo " full path)" >&2
        else
            printf '  %s\n' "${path}" >&2
        fi
    done
    echo "" >&2
    echo "Fix: add one row per file above to plans/ARCHIVE_MANIFEST.md, in the" >&2
    echo "section for its directory. Paste-ready rows:" >&2
    for path in "${missing[@]}"; do
        # shellcheck disable=SC2016  # literal backticks: this is a paste-ready manifest row
        printf '  | `%s` | (describe) |\n' "${path}" >&2
    done
    echo "" >&2
    echo "If a file landed in plans/.archive/ by mistake (e.g. 'plans-manager.sh" >&2
    echo "archive adr' moved a live ADR), move it back in the same commit instead." >&2
    exit 1
fi

echo "ok: archive manifest complete (${#disk_files[@]} files under plans/.archive/, ${#listed_set[@]} enumerated)"
