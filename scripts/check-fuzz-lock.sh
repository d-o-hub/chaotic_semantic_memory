#!/usr/bin/env bash
# Check that fuzz/Cargo.lock matches what the fuzz manifest resolves to today.
#
# WHY THIS EXISTS (issue #948, measured 2026-10-28 on PR #947): `fuzz/` is a
# separate workspace with its OWN lockfile, so the root `Cargo.lock` says nothing
# about it. The only gate was the CI `Fuzz Workspace Build` job
# (`cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked`);
# `scripts/validate.sh` never mentioned `fuzz` at all, so adding `postcard` to a
# crate the fuzz targets resolved gave `VALIDATE_RC=0` locally and a red CI job.
# This is a lockfile-only probe: no compile, ~1.5 s measured on this tree, and it
# fails on exactly the drift that job catches.
#
# Usage: check-fuzz-lock.sh [manifest-path]   (default: <repo-root>/fuzz/Cargo.toml)
# Exit:  0 lock is fresh, 1 lock does not match the manifest, 2 bad invocation or
#        the probe itself could not run (never reported as a lock verdict).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST="${1:-${SCRIPT_DIR}/../fuzz/Cargo.toml}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found; the fuzz lock check cannot run" >&2
  exit 2
fi
if [[ ! -f "${MANIFEST}" ]]; then
  echo "error: no fuzz manifest at ${MANIFEST}" >&2
  exit 2
fi

# `--locked` forbids the lockfile update cargo would otherwise perform silently.
# The drift message is cargo's "cannot update the lock file … because --locked was
# passed" (older cargo worded it "lock file needs to be updated but --locked was
# passed", which is what the #947 CI log showed).
#
# Exit codes are kept distinct on purpose: cargo reports its own failures as 101,
# while a tool that exists but cannot be executed fails with 126/127 from the
# shell. Collapsing those into "stale lockfile" would send a developer to
# regenerate a lock that was never the problem.
CARGO_RC=0
CARGO_ERR="$(cargo metadata --locked --format-version 1 --manifest-path "${MANIFEST}" 2>&1 >/dev/null)" ||
  CARGO_RC=$?
if [[ "${CARGO_RC}" -eq 0 ]]; then
  echo "fuzz lock check: ${MANIFEST} resolves against its committed lockfile"
  exit 0
fi
# Indented with a builtin herestring, not `sed`: a probe that reports on its own
# toolchain must not depend on one more external binary to do it.
while IFS= read -r line; do
  if [[ -n "${line}" ]]; then echo "  ${line}" >&2; fi
done <<<"${CARGO_ERR}"
if [[ "${CARGO_RC}" -eq 101 ]]; then
  echo "error: the lockfile next to ${MANIFEST} does not resolve against that manifest" >&2
  echo "  fix: cargo generate-lockfile --manifest-path ${MANIFEST}" >&2
  echo "  (the CI 'Fuzz Workspace Build' job fails on the same drift)" >&2
  exit 1
fi
echo "error: the fuzz lock probe could not be run (cargo exited ${CARGO_RC}); this is not a lock verdict" >&2
exit 2
