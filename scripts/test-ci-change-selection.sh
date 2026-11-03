#!/usr/bin/env bash
# Fixture for scripts/ci-change-selection.sh (issue #992).
#
# Table-driven contract: each row is a NUL-separated changed-path set and the
# seven expected outputs in workflow order
# (duckdb wasm fuzz mcp cli_cross mutation rust_src). The last two cases replay
# a real git diff and assert the workflow actually calls the shared classifier.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/ci-change-selection.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT
PASS=0
FAIL=0
PATHS="${TMP}/paths0"

set_paths() {
  : > "${PATHS}"
  for path in "$@"; do printf '%s\0' "${path}" >> "${PATHS}"; done
}

run_selector() {
  OUT="$(bash "${SUBJECT}" "$@" 2>"${TMP}/stderr.log")" && RC=0 || RC=$?
}

# expect <label> <expected seven values> <path...>
expect_paths() {
  local label="$1" expected="$2"
  shift 2
  set_paths "$@"
  run_selector --paths0 "${PATHS}"
  local actual
  actual="$(printf '%s\n' "${OUT}" | cut -d= -f2 | paste -sd' ' -)"
  if [[ "${RC}" -eq 0 && "${actual}" == "${expected}" ]]; then
    PASS=$((PASS + 1))
    printf '✅ %s\n' "${label}"
  else
    FAIL=$((FAIL + 1))
    printf '❌ %s: expected "%s", got "%s" (rc %s)\n' "${label}" "${expected}" "${actual}" "${RC}" >&2
    cat "${TMP}/stderr.log" | sed 's/^/  /' >&2
  fi
}

# ── Rust sources and manifests: every specialised job ─────────────
expect_paths "src/cli/mcp.rs selects MCP and CLI" \
  "true true true true true true true" "src/cli/mcp.rs"
expect_paths "src/mcp/** selects MCP" \
  "true true true true true true true" "src/mcp/tools.rs"
expect_paths "MCP integration test selects MCP, not mutation" \
  "true true true true true false true" "tests/mcp_integration.rs"
expect_paths "streamable HTTP MCP test selects MCP" \
  "true true true true true false true" "tests/mcp_streamable_http_integration.rs"
expect_paths "root Cargo.toml selects MCP and every matrix" \
  "true true true true true false true" "Cargo.toml"
expect_paths "Cargo.lock selects every matrix" \
  "true true true true true false true" "Cargo.lock"
expect_paths "root source selects mutation" \
  "true true true true true true true" "src/framework.rs"
expect_paths "owner crate production source selects mutation" \
  "true true true true true true true" "crates/csm-retrieval/src/bm25.rs"
expect_paths "owner crate test is not a mutable target" \
  "true true true true true false true" "crates/csm-retrieval/tests/foo.rs"
expect_paths "build.rs is a rust input, not a mutant" \
  "true true true true true false true" "build.rs"
expect_paths "src/*_tests.rs is not mutation-visible" \
  "true true true true true false true" "src/framework_tests.rs"
expect_paths "benches are rust inputs, not production mutants" \
  "true true true true true false true" "benches/benchmark.rs"

# ── Narrow non-Rust inputs ───────────────────────────────────────
expect_paths "wasm package selects only wasm" \
  "false true false false false false false" "wasm/package.json"
expect_paths "wasm script selects wasm" \
  "false true false false false false false" "scripts/build-wasm.sh"
expect_paths "duckdb fixture selects duckdb" \
  "true false false false false false false" "crates/csm-duckdb/fixtures/sample.parquet"
expect_paths "fuzz corpus selects fuzz" \
  "false false true false false false false" "fuzz/corpus/seed"
expect_paths "cli npm package selects cli_cross" \
  "false false false false true false false" "cli-npm/package.json"

# ── Docs, plans, skills and tooling do not compile ───────────────
expect_paths "README is docs-only" \
  "false false false false false false false" "README.md"
expect_paths "docs are docs-only" \
  "false false false false false false false" "docs/architecture/x.md"
expect_paths "plans are docs-only" \
  "false false false false false false false" "plans/ACTIONS.md"
expect_paths "skills are docs-only" \
  "false false false false false false false" ".agents/skills/x/SKILL.md"
expect_paths "scripts are tooling, not build inputs" \
  "false false false false false false false" "scripts/validate.sh"
expect_paths "a path with spaces is still classified" \
  "false false false false false false false" "docs/weird name.md"

# ── Build environment and workflow: conservative full selection ──
expect_paths "flake.nix enables every specialised job" \
  "true true true true true false false" "flake.nix"
expect_paths "ci.yml enables every specialised job" \
  "true true true true true false false" ".github/workflows/ci.yml"
expect_paths "an unclassified new path is conservative" \
  "true true true true true false false" "newdir/unknown.bin"

# ── Fallbacks ────────────────────────────────────────────────────
: > "${PATHS}"
run_selector --paths0 "${PATHS}"
empty_actual="$(printf '%s\n' "${OUT}" | cut -d= -f2 | paste -sd' ' -)"
if [[ "${RC}" -eq 0 && "${empty_actual}" == "true true true true true true true" ]]; then
  PASS=$((PASS + 1)); echo "✅ empty path set enables everything"
else
  FAIL=$((FAIL + 1)); echo "❌ empty path set: got '${empty_actual}' (rc ${RC})" >&2
fi

run_selector --all
all_actual="$(printf '%s\n' "${OUT}" | cut -d= -f2 | paste -sd' ' -)"
if [[ "${RC}" -eq 0 && "${all_actual}" == "true true true true true true true" ]]; then
  PASS=$((PASS + 1)); echo "✅ --all enables everything"
else
  FAIL=$((FAIL + 1)); echo "❌ --all: got '${all_actual}' (rc ${RC})" >&2
fi

set_paths "README.md"
run_selector --paths0 "${PATHS}" --trunk
trunk_actual="$(printf '%s\n' "${OUT}" | cut -d= -f2 | paste -sd' ' -)"
if [[ "${RC}" -eq 0 && "${trunk_actual}" == "true true true true true false false" ]]; then
  PASS=$((PASS + 1)); echo "✅ --trunk keeps full coverage and mutation off"
else
  FAIL=$((FAIL + 1)); echo "❌ --trunk docs: got '${trunk_actual}' (rc ${RC})" >&2
fi

set_paths "src/lib.rs"
run_selector --paths0 "${PATHS}" --trunk
trunk_src="$(printf '%s\n' "${OUT}" | cut -d= -f2 | paste -sd' ' -)"
if [[ "${RC}" -eq 0 && "${trunk_src}" == "true true true true true false true" ]]; then
  PASS=$((PASS + 1)); echo "✅ --trunk rust sets rust_src and keeps mutation off"
else
  FAIL=$((FAIL + 1)); echo "❌ --trunk rust: got '${trunk_src}' (rc ${RC})" >&2
fi

# ── github-output mode and error directions ─────────────────────
set_paths "src/cli/mcp.rs"
GITHUB_OUTPUT="${TMP}/github-output" run_selector --paths0 "${PATHS}" --github-output
if [[ "${RC}" -eq 0 && -z "${OUT}" ]] && grep -qx 'mcp=true' "${TMP}/github-output" && grep -qx 'mutation=true' "${TMP}/github-output"; then
  PASS=$((PASS + 1)); echo "✅ --github-output writes the keys and keeps stdout empty"
else
  FAIL=$((FAIL + 1)); echo "❌ --github-output mode" >&2
fi

run_selector --paths0 "${TMP}/does-not-exist"
if [[ "${RC}" -eq 2 ]]; then PASS=$((PASS + 1)); echo "✅ missing path file is exit 2"; else FAIL=$((FAIL + 1)); echo "❌ missing path file rc ${RC}" >&2; fi

run_selector --bogus
if [[ "${RC}" -eq 2 ]]; then PASS=$((PASS + 1)); echo "✅ unknown argument is exit 2"; else FAIL=$((FAIL + 1)); echo "❌ unknown argument rc ${RC}" >&2; fi

# ── End-to-end: a real git diff feeds the classifier ────────────
E2E="${TMP}/e2e"
mkdir -p "${E2E}/src/cli" "${E2E}/tests"
git -C "${E2E}" init -q
git -C "${E2E}" config user.email fixture@example.invalid
git -C "${E2E}" config user.name fixture
printf 'seed\n' > "${E2E}/README.md"
git -C "${E2E}" add README.md
git -C "${E2E}" commit -qm seed
printf 'fn main() {}\n' > "${E2E}/src/cli/mcp.rs"
printf '#[test] fn t() {}\n' > "${E2E}/tests/mcp_integration.rs"
git -C "${E2E}" add src tests
git -C "${E2E}" commit -qm change
git -C "${E2E}" diff --name-only -z HEAD~1 HEAD > "${TMP}/e2e-paths0"
run_selector --paths0 "${TMP}/e2e-paths0"
e2e_actual="$(printf '%s\n' "${OUT}" | cut -d= -f2 | paste -sd' ' -)"
if [[ "${RC}" -eq 0 && "${e2e_actual}" == "true true true true true true true" ]]; then
  PASS=$((PASS + 1)); echo "✅ end-to-end git diff selects MCP and mutation"
else
  FAIL=$((FAIL + 1)); echo "❌ end-to-end: got '${e2e_actual}' (rc ${RC})" >&2
fi

# ── Workflow wiring: the shared classifier owns the contract ────
CI_YML="${SCRIPT_DIR}/../.github/workflows/ci.yml"
if grep -q 'scripts/ci-change-selection.sh --paths0' "${CI_YML}" \
  && grep -q 'scripts/ci-change-selection.sh --all' "${CI_YML}" \
  && ! grep -q '^          match() {' "${CI_YML}"; then
  PASS=$((PASS + 1)); echo "✅ ci.yml calls the shared classifier and no longer embeds match()"
else
  FAIL=$((FAIL + 1)); echo "❌ ci.yml still embeds its own path classifier" >&2
fi

echo "ci-change-selection fixture: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
