#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EXPECTED="${SCRIPT_DIR}/SCRIPT_CATALOG.md"
RENDERED="$(mktemp "${TMPDIR:-/tmp}/script-catalog.XXXXXX")"
trap 'rm -f "$RENDERED"' EXIT

python3 "${SCRIPT_DIR}/gen_script_catalog.py" --root "$ROOT" --out "$RENDERED" >/dev/null
if ! cmp -s "$EXPECTED" "$RENDERED"; then
  diff -u "$EXPECTED" "$RENDERED" || true
  printf 'script catalog is stale; run scripts/gen_script_catalog.py\n' >&2
  exit 1
fi
python3 "${SCRIPT_DIR}/test-script-catalog.py"

# Referenced-by assertion (issue #1069): every scripts/ entry is wired
# (caller or fixture) or carries a reviewed scripts/SCRIPT_WIRING.md row
# that still matches reality — orphaned entries and stale assertions both fail.
python3 "${SCRIPT_DIR}/gen_script_catalog.py" --root "$ROOT" --check
printf 'script catalog drift gate: pass\n'
