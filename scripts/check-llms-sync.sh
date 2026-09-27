#!/usr/bin/env bash
# check-llms-sync.sh - Fail when committed llms.txt / llms-full.txt are stale.
#
# Snapshots both generated files, regenerates them via scripts/gen-llms-txt.sh,
# and compares. Regenerated files stay on disk for review; a nonzero exit means
# the committed versions drifted from the manifests (e.g. a dependency bump
# without a regeneration commit). validate.sh runs this as a gate.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

GENERATED_FILES=(llms.txt llms-full.txt)

cd "${PROJECT_ROOT}"

SNAPSHOT_DIR="$(mktemp -d)"
trap 'rm -rf "${SNAPSHOT_DIR}"' EXIT

# Snapshot first: a missing file is a hard failure, never an empty baseline
# that would report every regeneration as a change.
for file in "${GENERATED_FILES[@]}"; do
    if [[ ! -f "${file}" ]]; then
        echo "missing generated file: ${file}" >&2
        exit 1
    fi
    cp -- "${file}" "${SNAPSHOT_DIR}/${file}"
done

# Regenerate in place; propagate generator failures instead of reporting success.
gen_status=0
"${SCRIPT_DIR}/gen-llms-txt.sh" || gen_status=$?
if [[ "${gen_status}" -ne 0 ]]; then
    echo "generator failed: scripts/gen-llms-txt.sh (exit ${gen_status})" >&2
    exit "${gen_status}"
fi

stale=0
for file in "${GENERATED_FILES[@]}"; do
    if ! cmp -s "${SNAPSHOT_DIR}/${file}" "${file}"; then
        echo "stale generated file: ${file}"
        stale=1
    fi
done

if [[ "${stale}" -ne 0 ]]; then
    echo "❌ committed llms files are stale; commit the regenerated versions" >&2
    exit 1
fi

echo "✅ llms.txt and llms-full.txt are up to date"
