#!/usr/bin/env bash
# scripts/crates-io-index-path.sh — the real sparse-index path for a crate name.
#
# Issue #974: the publish-side helpers did `p1="${name:0:2}" p2="${name:2:2}"`
# unconditionally, which is right only for names of 4+ characters. The live
# index measured 2026-10-31: `1/a` and `2/io` exist while `1/a/a` and `2/io/io`
# do not, and `3/s/syn` exists while `sy/n/syn` does not. Every crate published
# today has a long name, so nothing is broken now; one shared implementation
# prevents a 1-3 character crate from reading as permanently unpublished.
#
# Usage: crates-io-index-path.sh <crate-name>
# Exit: 0 path on stdout, 2 usage error.
# Fixture: scripts/test-crates-io-index-path.sh
set -euo pipefail

if [[ "$#" -ne 1 || -z "${1:-}" ]]; then
    echo "usage: crates-io-index-path.sh <crate-name>" >&2
    exit 2
fi
name="$1"
case "${#name}" in
    1) printf '1/%s\n' "${name}" ;;
    2) printf '2/%s\n' "${name}" ;;
    3) printf '3/%s/%s\n' "${name:0:1}" "${name}" ;;
    *) printf '%s/%s/%s\n' "${name:0:2}" "${name:2:2}" "${name}" ;;
esac
