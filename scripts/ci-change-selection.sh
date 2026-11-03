#!/usr/bin/env bash
# scripts/ci-change-selection.sh — deterministic path classifier for ci.yml.
#
# Issue #992: the PR-specific `detect-changes` filter was a block of shell
# regexes embedded in the workflow, with no contract test. It selected the MCP
# feature job only for `src/mcp/**`, so a change to `src/cli/mcp.rs` or
# `tests/mcp*_integration.rs` silently skipped the dedicated MCP job.
#
# Contract (direct-path applicability only):
#   * A changed Rust source, manifest, lockfile, build script or toolchain/build
#     input enables every specialised job, because those jobs compile workspace
#     code whose transitive graph cannot be derived from a path alone. This is
#     deliberately conservative; it does NOT claim semantic dependency isolation.
#   * A docs/plans/skills-only diff enables none of them.
#   * An unclassified path (a new top-level file or directory) enables every
#     specialised job rather than assuming it is harmless.
#   * Mutation is narrower: only production Rust under `src/**` or
#     `crates/*/src/**`, excluding tests and manifests.
#   * A missing base / failed diff is the caller's problem: call `--all`, which
#     must be the fallback, never a silent skip.
#
# Usage:
#   ci-change-selection.sh --paths0 <file> [--trunk] [--all] [--github-output]
#
# `--paths0` reads NUL-separated paths (git diff --name-only -z).
# `--trunk` applies the push-to-main policy (all specialised jobs on; mutation off).
# `--all` forces every output true (used when the diff cannot be computed).
# `--github-output` appends `key=value` lines to $GITHUB_OUTPUT instead of stdout.
# Exit: 0 classified, 2 unreadable input / usage error.
# Fixture: scripts/test-ci-change-selection.sh
set -euo pipefail

PATHS_FILE=""
TRUNK=0
FORCE_ALL=0
GITHUB_OUTPUT_MODE=0
KEYS=(duckdb wasm fuzz mcp cli_cross mutation rust_src)

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --paths0)
      [[ "$#" -ge 2 ]] || { echo "error: --paths0 needs a file" >&2; exit 2; }
      PATHS_FILE="$2"
      shift 2
      ;;
    --trunk) TRUNK=1; shift ;;
    --all) FORCE_ALL=1; shift ;;
    --github-output) GITHUB_OUTPUT_MODE=1; shift ;;
    -h|--help)
      sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ "${GITHUB_OUTPUT_MODE}" -eq 1 && -z "${GITHUB_OUTPUT:-}" ]]; then
  echo "error: --github-output requires GITHUB_OUTPUT" >&2
  exit 2
fi

declare -A OUT=()
for key in "${KEYS[@]}"; do OUT["${key}"]=false; done

emit() {
  if [[ "${GITHUB_OUTPUT_MODE}" -eq 1 ]]; then
    for key in "${KEYS[@]}"; do
      printf '%s=%s\n' "${key}" "${OUT[${key}]}" >> "${GITHUB_OUTPUT}"
    done
  else
    for key in "${KEYS[@]}"; do
      printf '%s=%s\n' "${key}" "${OUT[${key}]}"
    done
  fi
}

if [[ "${FORCE_ALL}" -eq 1 ]]; then
  for key in "${KEYS[@]}"; do OUT["${key}"]=true; done
  emit
  exit 0
fi

if [[ -z "${PATHS_FILE}" || ! -r "${PATHS_FILE}" ]]; then
  echo "error: --paths0 file is missing or unreadable: ${PATHS_FILE:-<none>}" >&2
  exit 2
fi

PATHS=()
while IFS= read -r -d '' path; do
  [[ -n "${path}" ]] && PATHS+=("${path}")
done < "${PATHS_FILE}"

# An empty diff is "no information", not "nothing changed": enable everything
# rather than skipping a gate on a first push or a truncated collection.
if [[ "${#PATHS[@]}" -eq 0 ]]; then
  for key in "${KEYS[@]}"; do OUT["${key}"]=true; done
  echo "ci-change-selection: empty path set -> all jobs enabled" >&2
  emit
  exit 0
fi

any_rust=0
any_production_rust=0
unknown=0

for raw in "${PATHS[@]}"; do
  path="${raw#./}"
  classified=0

  # Documentation, plans, skills and progress do not compile.
  case "${path}" in
    *.md|docs/*|book/*|plans/*|progress/*|LICENSE*|README*|SECURITY*|CHANGELOG*|.agents/*|.github/ISSUE_TEMPLATE/*|.github/PULL_REQUEST_TEMPLATE*|.github/dependabot.yml)
      classified=1
      ;;
  esac

  # Tooling that does not feed a build: the selector, hooks, fixtures and the
  # optional diagnostics. `ci.yml` itself is handled below (it defines the jobs).
  case "${path}" in
    scripts/*|.githooks/*|.github/actions/*)
      classified=1
      ;;
  esac

  # Rust sources, manifests, lockfiles, build scripts and toolchain/build inputs.
  case "${path}" in
    *.rs|*/Cargo.toml|Cargo.toml|Cargo.lock|*/Cargo.lock|rust-toolchain*|.cargo/*)
      any_rust=1
      classified=1
      ;;
  esac

  # Production Rust is the only thing cargo-mutants can mutate.
  if [[ "${path}" =~ ^(src|crates/[^/]+/src)/.*\.rs$ ]] \
    && [[ ! "${path}" =~ (^|/)(tests/|tests\.rs$|[^/]*_tests\.rs$) ]]; then
    any_production_rust=1
  fi

  # A workflow change can alter any job; a build-environment change can alter
  # every compilation. Both are conservative full-selection inputs.
  case "${path}" in
    .github/workflows/*|flake.nix|flake.lock|rust-toolchain*|.cargo/*)
      classified=1
      unknown=1
      ;;
  esac

  # Direct non-Rust inputs for a specialised job (a manifest change is already
  # covered by any_rust above).
  case "${path}" in
    wasm/*|crates/csm-wasm/*|scripts/build-wasm.sh|scripts/check-wasm-freshness.sh|scripts/wasm_size_gate.sh)
      classified=1
      OUT[wasm]=true
      ;;
    fuzz/*)
      classified=1
      OUT[fuzz]=true
      ;;
    crates/csm-duckdb/*)
      classified=1
      OUT[duckdb]=true
      ;;
    cli-npm/*|crates/csm-cli/*)
      classified=1
      OUT[cli_cross]=true
      ;;
    tests/mcp*)
      classified=1
      OUT[mcp]=true
      ;;
  esac

  if [[ "${classified}" -eq 0 ]]; then
    unknown=1
    echo "ci-change-selection: unclassified path '${path}' -> all jobs enabled" >&2
  fi
done

if [[ "${TRUNK}" -eq 1 ]]; then
  for key in duckdb wasm fuzz mcp cli_cross; do OUT["${key}"]=true; done
  OUT[mutation]=false
  [[ "${any_rust}" -eq 1 ]] && OUT[rust_src]=true
  emit
  exit 0
fi

if [[ "${any_rust}" -eq 1 || "${unknown}" -eq 1 ]]; then
  for key in duckdb wasm fuzz mcp cli_cross; do OUT["${key}"]=true; done
fi
[[ "${any_rust}" -eq 1 ]] && OUT[rust_src]=true
[[ "${any_production_rust}" -eq 1 ]] && OUT[mutation]=true

emit
