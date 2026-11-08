#!/usr/bin/env bash
# clean-incomplete-duckdb-unpack.sh — repair a cached libduckdb-sys source tree.
#
# WHY THIS EXISTS (issue #1004)
# libduckdb-sys unpacks its bundled DuckDB sources into
#   <target>/<profile>/build/libduckdb-sys-<hash>/out/duckdb-sources/<archive-hash>/
# and marks a completed tree with `.complete`. The unpack stages into a temporary
# directory and publishes with `fs::rename`, which fails with DirectoryNotEmpty
# when the destination exists without that marker — an interrupted unpack, or a
# tree left by an older crate version inside a restored CI cache. The build then
# panics in build_bundled_cc.rs before any test runs:
#   unpacking bundled DuckDB sources: Os { code: 39, kind: DirectoryNotEmpty }
# Re-running the job is not a fix: it restores the same cache. The archive lives
# in the cargo registry, so deleting an incomplete tree is safe and costs only the
# C++ rebuild that the broken tree would have forced anyway.
#
# The same cache key is shared by several jobs with different feature sets, so a
# tree that is valid for one build can be invalid for the next. Every job that
# compiles the workspace should run this after restoring its cache and before
# invoking cargo.
#
# Usage: clean-incomplete-duckdb-unpack.sh [TARGET_DIR]
#   TARGET_DIR defaults to $CARGO_TARGET_DIR, then `target`.
# Exit codes: 0 repaired or nothing to repair; 2 unreadable target directory.
set -euo pipefail

TARGET_DIR="${1:-${CARGO_TARGET_DIR:-target}}"

if [[ ! -d "${TARGET_DIR}" ]]; then
    echo "duckdb unpack repair: ${TARGET_DIR} does not exist; nothing to clean"
    exit 0
fi
if [[ ! -r "${TARGET_DIR}" ]]; then
    echo "duckdb unpack repair: ${TARGET_DIR} is not readable" >&2
    exit 2
fi

removed=0
# NUL-safe: a custom CARGO_TARGET_DIR can contain spaces.
while IFS= read -r -d '' root; do
    # A killed unpack leaves its staging directory behind; Drop never runs.
    while IFS= read -r -d '' staging; do
        echo "duckdb unpack repair: removing abandoned staging ${staging}"
        rm -rf -- "${staging}"
    done < <(find "${root}" -mindepth 1 -maxdepth 1 -type d -name '.duckdb-unpack-*' -print0)

    while IFS= read -r -d '' tree; do
        if [[ -f "${tree}/.complete" ]]; then
            continue
        fi
        echo "duckdb unpack repair: removing incomplete source tree ${tree}"
        rm -rf -- "${tree}"
        removed=$((removed + 1))
    done < <(find "${root}" -mindepth 1 -maxdepth 1 -type d -print0)
done < <(find "${TARGET_DIR}" -type d -name duckdb-sources -print0 2>/dev/null)

echo "duckdb unpack repair: removed ${removed} incomplete source tree(s)"
