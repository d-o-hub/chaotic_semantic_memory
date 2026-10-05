#!/usr/bin/env bash
# check-skill-catalog.sh - fail when the committed skill catalog is stale.
#
# Snapshots .agents/skills/CATALOG.md, re-renders it with
# scripts/gen-skill-catalog.sh into a temp directory, and compares. A nonzero
# exit means the committed catalog no longer describes the skills on disk (e.g.
# a skill was added, renamed or removed without a regeneration commit) - exactly
# the drift issue #828 found: the file claimed "32 skills." against 33 on disk
# and omitted `pr-roast-triage`, the skill AGENTS.md's roast gate depends on.
#
# WHY A SEPARATE SCRIPT AND NOT A --check FLAG ON THE GENERATOR
# This repo already splits generator from checker (scripts/gen-llms-txt.sh vs
# scripts/check-llms-sync.sh) so a PR run can verify without touching the
# working tree; rendering into a temp dir keeps that property instead of leaving
# a regenerated file behind for the author to notice.
#
# FAIL-CLOSED, THREE WAYS
#   - a generator error propagates its own exit status instead of printing
#     "up to date";
#   - the committed catalog is snapshotted before anything is regenerated, so a
#     missing file is reported as missing, never compared against an empty
#     baseline that could pass by accident;
#   - the generator itself is required to exist and be executable.
#
# No network, no cargo, no write to the working tree.
# Usage: scripts/check-skill-catalog.sh
# Negative fixture: scripts/test-skill-catalog-gate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

readonly CATALOG=".agents/skills/CATALOG.md"
readonly GENERATOR="${SCRIPT_DIR}/gen-skill-catalog.sh"

cd "${PROJECT_ROOT}"

if [[ ! -x "${GENERATOR}" ]]; then
    printf 'generator missing or not executable: %s\n' "${GENERATOR#"${PROJECT_ROOT}/"}" >&2
    exit 1
fi

# Snapshot first: a missing committed catalog is a hard failure, never an empty
# baseline that would report every regeneration as a change.
if [[ ! -f "${CATALOG}" ]]; then
    printf 'missing generated file: %s\n' "${CATALOG}" >&2
    exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

expected="${WORK_DIR}/CATALOG.md"
rendered="${WORK_DIR}/generator.log"

# Regenerate out of tree; propagate generator failures instead of reporting success.
gen_status=0
bash "${GENERATOR}" --root "${PROJECT_ROOT}" --out "${expected}" > "${rendered}" 2>&1 || gen_status=$?
if [[ "${gen_status}" -ne 0 ]]; then
    printf 'generator failed: scripts/gen-skill-catalog.sh (exit %s)\n' "${gen_status}" >&2
    sed 's/^/  /' "${rendered}" >&2
    exit "${gen_status}"
fi

if ! cmp -s "${CATALOG}" "${expected}"; then
    printf 'stale generated file: %s\n' "${CATALOG}"
    printf 'committed vs regenerated (first 20 diff lines):\n'
    diff -u --label "${CATALOG}" --label "${CATALOG} (regenerated)" \
        "${CATALOG}" "${expected}" | head -n 20 || true
    printf '❌ %s is stale; run: scripts/gen-skill-catalog.sh\n' "${CATALOG}" >&2
    exit 1
fi

skill_count="$(grep -c '^| `' "${expected}" || true)"
printf '✅ %s matches the tree (%s skills)\n' "${CATALOG}" "${skill_count}"
