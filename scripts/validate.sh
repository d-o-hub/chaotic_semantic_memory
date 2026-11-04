#!/usr/bin/env bash
set -euo pipefail
export CSM_VALIDATION_REQUIRED=true

# Source lint caching library for faster repeated runs
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${SCRIPT_DIR}/lib/lint_cache.sh" ]]; then
    source "${SCRIPT_DIR}/lib/lint_cache.sh"
fi

MAX_SRC_LOC=500
WASM_TARGET="wasm32-unknown-unknown"

echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

# Test-attribute gate: a test body without `#[test]` compiles, lints clean and
# simply never runs (no dead_code warning in a --test build), so no Rust sensor
# can catch it. Textual scan instead — cheapest stage here, so it runs first.
# The negative fixture is invoked too: a gate whose failure mode is untested can
# rot into a pass-for-the-wrong-reason check (see progress/LEARNINGS.md 2026-09-30).
if [[ -x "${SCRIPT_DIR}/check-test-attributes.sh" ]]; then
  echo "==> Test-attribute gate (unregistered test bodies in tests/)"
  "${SCRIPT_DIR}/check-test-attributes.sh"
  echo "==> Test-attribute gate fixture"
  "${SCRIPT_DIR}/test-check-test-attributes.sh"
else
  echo "Error: scripts/check-test-attributes.sh missing or not executable"
  exit 1
fi

echo "==> Coverage inventory attribute fixture"
"${SCRIPT_DIR}/test-coverage-report-inventory.sh"

echo "==> Generated script catalog and fixture gate"
"${SCRIPT_DIR}/check-script-catalog.sh"

echo "==> Self-fix-loop safety fixtures"
"${SCRIPT_DIR}/test-self-fix-loop-safety.sh"

echo "==> GOAP orchestrator mutation safety fixtures"
"${SCRIPT_DIR}/test-goap-orchestrator-safety.sh"

# CI crate-matrix gate (issue #827): the `test-workspace-crates` matrix must be
# derived from `cargo metadata`, every exclusion must have a dedicated job in
# ci.yml, and an empty derivation must fail. Cheap (one `cargo metadata
# --no-deps`, no network), and it runs the fixture too so the failure modes stay
# proven in both directions.
if [[ -x "${SCRIPT_DIR}/ci-workspace-crate-matrix.sh" ]]; then
  echo "==> CI crate-matrix gate (workspace vs .github/workflows/ci.yml)"
  "${SCRIPT_DIR}/ci-workspace-crate-matrix.sh" --check
  echo "==> CI crate-matrix gate fixture"
  "${SCRIPT_DIR}/test-ci-workspace-crate-matrix.sh"
else
  echo "Error: scripts/ci-workspace-crate-matrix.sh missing or not executable"
  exit 1
fi

# Git hooks validation & bootstrap fixture (issue #842).
# Ensures hooks are installed, core.hooksPath is set to .githooks, and required
# hooks exist and are executable. Runs the fixture to verify all failure directions.
if [[ -x "${SCRIPT_DIR}/validate-git-hooks.sh" ]]; then
  echo "==> Git hooks configuration validation"
  "${SCRIPT_DIR}/validate-git-hooks.sh" --check
  echo "==> Git hooks bootstrap fixture"
  "${SCRIPT_DIR}/test-hook-bootstrap.sh"
else
  echo "Error: scripts/validate-git-hooks.sh missing or not executable" >&2
  exit 1
fi

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> cargo deny --locked check"
# cargo-deny fetches the RustSec advisory-db by spawning the host `git` CLI, and
# the CI lint job reaches this script through its LD_LIBRARY_PATH wrapper, so the
# host git-remote-https resolves libdl/libpthread against nix-store glibc-2.44 and
# dies on GLIBC_ABI_DT_X86_64_PLT (lint job 122979314258). The standalone Cargo
# Deny job runs the identical command on the identical devShell without that
# export and passes — the A/B that localises the failure to the environment
# rather than the runner image.
env -u LD_LIBRARY_PATH cargo deny --locked check

# `fuzz/` is a second workspace with its own Cargo.lock, so nothing above says
# whether it still matches its manifest: a dependency added to a crate the fuzz
# targets resolve passed this script with rc 0 and failed CI's Fuzz Workspace
# Build job (PR #947, issue #948). Resolution only, no compile.
echo "==> fuzz/Cargo.lock freshness (issue #948)"
"${SCRIPT_DIR}/check-fuzz-lock.sh"

# `GitHub Pages` runs `cargo doc --no-deps --all-features` with
# CARGO_BUILD_WARNINGS=deny exported by actions-rust-lang/setup-rust-toolchain, so
# a rustdoc warning fails nothing a PR can see and instead silently stops the docs
# site deploying: 6 unresolved-link warnings from one `src/cli/args.rs` doc line
# failed four consecutive main pushes (34751139063, 35274354770, 35614812503,
# 37126532546). Same command, same flags, so the failure surfaces at PR time.
echo "==> cargo doc --no-deps --all-features (rustdoc warnings denied)"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# CI applies stricter RUSTFLAGS; this is the minimal local gate
# Check for warnings AND ensure compilation succeeds
echo "==> cargo test --no-run --all-features (check for warnings)"
OUTPUT=$(cargo test --no-run --all-features 2>&1) || {
  echo "Error: Compilation failed with --all-features"
  echo "$OUTPUT"
  exit 1
}
if echo "$OUTPUT" | grep -qi "warning:"; then
  echo "Error: Warnings found in test compilation"
  echo "$OUTPUT" | grep -i "warning:"
  exit 1
fi

echo "==> cargo test --all-targets --all-features"
cargo test --all-targets --all-features

echo "==> Source file LOC gate (< ${MAX_SRC_LOC})"
while IFS= read -r -d '' file; do
  loc="$(wc -l < "${file}")"
  if [[ "${loc}" -gt "${MAX_SRC_LOC}" ]]; then
    echo "LOC gate failed: ${file} has ${loc} lines"
    exit 1
  fi
  # Use lint caching if available
  if declare -f lint_cache_needs_check &>/dev/null; then
    if lint_cache_needs_check "${file}"; then
      lint_cache_mark_checked "${file}"
    fi
  fi
  echo "ok: ${file} (${loc} LOC)"
done < <(find src crates -type f -name '*.rs' -not -path '*/target/*' -print0)

if rustup target list --installed | grep -q "^${WASM_TARGET}\$"; then
  # -p csm-wasm, not a bare `--features wasm`: the root manifest's default-members
  # now covers every member (issue #905), so a bare command selects the whole
  # workspace and dies on tokio's wasm feature gate ("Only features sync,macros,
  # io-util,rt,time are supported on wasm"). This is the same check the CI wasm
  # job runs (ci.yml: `cargo check --target wasm32-unknown-unknown -p csm-wasm`).
  echo "==> cargo check --target ${WASM_TARGET} -p csm-wasm"
  cargo check --target "${WASM_TARGET}" -p csm-wasm
else
  echo "skip: ${WASM_TARGET} target not installed"
fi

if [[ -x scripts/wasm_size_gate.sh ]]; then
  # The gate measures the release/web package, which needs wasm-pack and the
  # wasm32 target. The CI wasm job runs it against the package it built and
  # smoke-tested; here it only runs when the toolchain is present, so the lint
  # job does not fail on a missing build tool.
  if command -v wasm-pack >/dev/null 2>&1; then
    echo "==> scripts/wasm_size_gate.sh"
    scripts/wasm_size_gate.sh
  else
    echo "skip: scripts/wasm_size_gate.sh (wasm-pack not installed; CI wasm job runs it)"
  fi
fi

echo "==> Generating/validating llms.txt and llms-full.txt"
scripts/check-llms-sync.sh

# Regression fixtures for the deterministic text gates above. Both suites build a
# throwaway mock tree, copy the checker in, and assert the checker's failure
# directions; they existed on main unrun by anything since they were committed
# (#829), so either gate could have rotted into passing for the wrong reason.
# `test-version-sync.sh` exercises the checker that `.github/workflows/
# version-integrity.yml:32` runs — the fixture has no CI coverage of its own
# otherwise, and it was not even executable (mode 664) until it was wired.
for fixture in test-llms-sync.sh test-version-sync.sh test-check-version-literals.sh; do
  if [[ -x "${SCRIPT_DIR}/${fixture}" ]]; then
    echo "==> Fixture: scripts/${fixture}"
    "${SCRIPT_DIR}/${fixture}"
  else
    echo "Error: scripts/${fixture} missing or not executable"
    exit 1
  fi
done

# Module-level mutation inventory parser (PR #861 review): pins the real
# cargo-mutants 27.1.0 outcomes.json schema (variant-name summaries) and the
# aggregate cross-check — the failure directions a lowercased compare hides.
echo "==> Mutation inventory fixture"
"${SCRIPT_DIR}/test-mutation-inventory.sh"

# Mutation gate classification (PR #977): pins the four ways "0 mutants" happens,
# the package scope that must reach generation (not only the tests), the per-arm
# feature split, and union-not-average scoring. All cargo/git calls are stubbed,
# so it costs seconds instead of a 40-minute sweep.
echo "==> Mutation gate classification fixture"
"${SCRIPT_DIR}/test-mutation-no-diff.sh"

# cfg-split declarations (issue #952): a native/wasm pair inside the mutation
# surface contributes an unkillable MISSED mutant. The checker is diff-scoped in
# the gate; the fixture proves both the finding and the untouched-file direction.
echo "==> cfg-split mutant fixture"
"${SCRIPT_DIR}/test-check-cfg-mutants.sh"

# Shard union arithmetic (issue #951): the aggregator must sum caught/viable
# across shards, never average per-shard scores, and must refuse missing shards.
echo "==> Mutation shard union fixture"
"${SCRIPT_DIR}/test-mutation-union.sh"

# Fuzz lock freshness (issue #948): the stale/fresh pair on a synthetic path-only
# workspace, and the fail-closed directions of scripts/check-fuzz-lock.sh — a
# missing manifest or an unusable cargo must not read as "lock is fresh".
echo "==> Fuzz lock fixture"
"${SCRIPT_DIR}/test-check-fuzz-lock.sh"

# Squash-subject length (issue #965): the boundary a title has to clear before
# GitHub appends " (#<n>)", the character-vs-byte unit commitlint actually enforces,
# and a replay of both ci.yml steps that turn a measurement into a check verdict —
# the fail-open shape (a skipped `if` branch leaves `$?` at 0) is only reachable
# through the live step body, so the fixture reads it out of the workflow file.
echo "==> Squash subject fixture"
"${SCRIPT_DIR}/test-check-squash-subject-length.sh"

# PR triage truthfulness (issue #993). `pr-triage.sh` is a read-only operator
# helper, so no Rust sensor can catch it turning a failed/empty/unknown check
# query into "all pass". The fixture drives every state and a head change against
# a fake gh, and counts the per-PR check queries.
if [[ -x "${SCRIPT_DIR}/test-pr-triage.sh" ]]; then
  echo "==> PR triage fixture (failed/empty/unknown queries are never green)"
  "${SCRIPT_DIR}/test-pr-triage.sh"
else
  echo "Error: scripts/test-pr-triage.sh missing or not executable" >&2
  exit 1
fi

LOC=$(grep -cE '^\s*(pub |fn |struct |enum |trait |impl )' llms-full.txt || true)
echo "Public API surface: $LOC symbols"

THRESHOLD=5000
if [[ "$LOC" -gt "$THRESHOLD" ]]; then
  echo "❌ API surface $LOC exceeds threshold of $THRESHOLD"
  exit 1
fi

echo "✅ API surface within threshold ($LOC / $THRESHOLD)"

if command -v npm >/dev/null 2>&1; then
  echo "==> CLI npm pack smoke test"
  cargo build --release -p chaotic_semantic_memory --bin csm
  mkdir -p cli-npm/bin
  # The build may land outside ./target when CARGO_TARGET_DIR is set (shared caches),
  # so resolve the artifact the same way cargo does (2026-10-10: this cp failed on a
  # workstation with a shared target dir while the build itself succeeded).
  TARGET_DIR="${CARGO_TARGET_DIR:-target}"
  cp "${TARGET_DIR}/release/csm" cli-npm/bin/csm-linux-x64
  chmod 755 cli-npm/bin/csm-linux-x64
  pushd cli-npm >/dev/null
  TARBALL=$(npm pack --silent)
  TMP_DIR=$(mktemp -d)
  npm install --prefix "$TMP_DIR" "./$TARBALL" >/dev/null
  "$TMP_DIR/node_modules/.bin/csm" --help >/dev/null
  rm -f "$TARBALL"
  popd >/dev/null
  rm -rf "$TMP_DIR"
  rm -f cli-npm/bin/csm-linux-x64
else
  echo "skip: npm not found, skipping CLI pack smoke test"
fi

mapfile -t SHELL_SCRIPTS < <(find "${SCRIPT_DIR}" -name '*.sh' -type f -print)
echo "==> Optional ShellCheck diagnostic"
"${SCRIPT_DIR}/run-shellcheck-optional.sh" "${SHELL_SCRIPTS[@]}"
echo "==> ShellCheck dependency fixture"
"${SCRIPT_DIR}/test-run-shellcheck-optional.sh"

# Markdownlint for all markdown files (optional - only if installed)
# Supports both markdownlint-cli (npm) and mdl (ruby)
if command -v markdownlint >/dev/null 2>&1; then
  echo "==> Markdownlint (markdownlint-cli)"
  MARKDOWN_FILES=$(find . -name '*.md' -type f -not -path './node_modules/*' -not -path './.git/*')
  if [[ -n "${MARKDOWN_FILES}" ]]; then
    markdownlint "${MARKDOWN_FILES}"
    echo "ok: all markdown files pass markdownlint"
  else
    echo "skip: no markdown files found"
  fi
elif command -v mdl >/dev/null 2>&1; then
  echo "==> Markdownlint (mdl)"
  MARKDOWN_FILES=$(find . -name '*.md' -type f -not -path './node_modules/*' -not -path './.git/*')
  if [[ -n "${MARKDOWN_FILES}" ]]; then
    mdl --style all "${MARKDOWN_FILES}"
    echo "ok: all markdown files pass mdl"
  else
    echo "skip: no markdown files found"
  fi
else
  echo "skip: markdownlint not installed (optional)"
  echo "      Install with: npm install -g markdownlint-cli || gem install mdl"
fi

# CHANGELOG structure — a version section must not declare one release-type heading twice.
# version-integrity.yml runs the checker only when a PR touches CHANGELOG.md, so a PR that
# weakens the checker itself is never scored; this is the path that always runs. The checker
# reads Cargo.toml and CHANGELOG.md from the CWD, so it runs from the repo root explicitly.
if [[ -x "${SCRIPT_DIR}/validate-changelog.sh" ]]; then
  echo "==> CHANGELOG structure validation (fail-closed)"
  ( cd "$(dirname "${SCRIPT_DIR}")" && "${SCRIPT_DIR}/validate-changelog.sh" )
  echo "==> CHANGELOG gate fixtures"
  "${SCRIPT_DIR}/test-validate-changelog.sh"
else
  echo "Error: scripts/validate-changelog.sh missing or not executable"
  exit 1
fi

# Documentation link and version checker (issue #840)
if [[ -x "${SCRIPT_DIR}/check-docs-links.sh" ]]; then
  echo "==> Documentation link & version validation (fail-closed)"
  "${SCRIPT_DIR}/check-docs-links.sh"
  echo "==> Documentation link gate fixture (both directions of every check)"
  "${SCRIPT_DIR}/test-check-docs-links.sh"
else
  echo "Error: scripts/check-docs-links.sh missing or not executable"
  exit 1
fi
# Pre-release gate verdict (issue #903). The verdict only ever renders on a main CI
# completion, so a wrong branch is invisible to every local gate; the fixture replays
# all three verdicts against scripts/pre-release-gate-summary.sh.
if [[ -x "${SCRIPT_DIR}/test-pre-release-gate-summary.sh" ]]; then
  echo "==> Pre-release gate verdict fixture (no-verdict / pass / fail)"
  "${SCRIPT_DIR}/test-pre-release-gate-summary.sh"
else
  echo "Error: scripts/test-pre-release-gate-summary.sh missing or not executable"
  exit 1
fi

# Release refusal detail (issue #960). A gate refusal only ever renders while a
# release is being blocked, which is the one moment nobody is debugging locally; the
# fixture replays every refusal shape (failed / in-flight / skipped / jobless /
# non-JSON / newline-in-name) against scripts/gate-failure-detail.sh.
if [[ -x "${SCRIPT_DIR}/test-gate-failure-detail.sh" ]]; then
  echo "==> Release refusal detail fixture (names the blocking gate job)"
  "${SCRIPT_DIR}/test-gate-failure-detail.sh"
else
  echo "Error: scripts/test-gate-failure-detail.sh missing or not executable"
  exit 1
fi

# Release verification replay (issues #954, #971, #972). `verify-release` and the
# `publish-crates` index probes are inline workflow shell: the mutation surface does
# not contain `.github/workflows/*.yml`, and the steps only run after a tag, so this
# fixture is their only test surface. It extracts the live step bodies and replays them
# under GitHub's own `bash -e`, with the registries stubbed.
if [[ -x "${SCRIPT_DIR}/test-verify-release-step.sh" ]]; then
  echo "==> Release verification fixture (every artifact, provenance, assets)"
  "${SCRIPT_DIR}/test-verify-release-step.sh"
else
  echo "Error: scripts/test-verify-release-step.sh missing or not executable"
  exit 1
fi

# Release verdict and index-path fixtures (issues #969/#974). The verdict script
# is the only place the four not-a-release / nothing-to-do / published / partial
# states are distinguished; the index-path fixture pins the 1/2/3-character
# crates.io layout measured on the live sparse index.
for fixture in test-release-verdict.sh test-crates-io-index-path.sh; do
  if [[ -x "${SCRIPT_DIR}/${fixture}" ]]; then
    echo "==> Fixture: scripts/${fixture}"
    "${SCRIPT_DIR}/${fixture}"
  else
    echo "Error: scripts/${fixture} missing or not executable" >&2
    exit 1
  fi
done

# Skill format validation (ADR-0096) — fail-closed LOC/frontmatter/local refs
if [[ -x "${SCRIPT_DIR}/validate-skill-format.sh" ]]; then
  echo "==> Skill format validation (fail-closed)"
  "${SCRIPT_DIR}/validate-skill-format.sh"
  echo "==> Skill format gate fixture (both directions of every check)"
  "${SCRIPT_DIR}/test-validate-skill-format.sh"
else
  echo "Error: scripts/validate-skill-format.sh missing or not executable"
  exit 1
fi

# Skill catalog drift gate (#828): .agents/skills/CATALOG.md is generated from
# the tree, so a stale or hand-edited copy is a hard failure — the committed file
# claimed "32 skills." against 33 on disk and omitted `pr-roast-triage`, the skill
# AGENTS.md's roast gate points at. The fixture runs too: a gate whose failure
# mode is untested can rot into a pass-for-the-wrong-reason check (see
# progress/LEARNINGS.md 2026-09-30).
if [[ -x "${SCRIPT_DIR}/check-skill-catalog.sh" ]]; then
  echo "==> Skill catalog drift gate"
  "${SCRIPT_DIR}/check-skill-catalog.sh"
  echo "==> Skill catalog drift gate fixture"
  "${SCRIPT_DIR}/test-skill-catalog-gate.sh"
else
  echo "Error: scripts/check-skill-catalog.sh missing or not executable"
  exit 1
fi

# Shared conventions validation (ADR-0090)
if [[ -x "${SCRIPT_DIR}/check-shared-conventions.sh" ]]; then
  echo "==> Shared conventions validation (fail-closed)"
  "${SCRIPT_DIR}/check-shared-conventions.sh"
  echo "==> Shared conventions gate fixture"
  "${SCRIPT_DIR}/test-check-shared-conventions.sh"
else
  echo "Error: scripts/check-shared-conventions.sh missing or not executable"
  exit 1
fi

# Workspace version-pin edge gate (#964): eighteen intra-workspace `version = "…"` pins
# drifted across three releases while scripts/verify-version-sync.sh printed
# "✓ All local versions synchronized: 0.3.9" — it compared member *own* versions and the
# root→member pins only, and sync-version.sh's rewrite sed was root-only too. Nothing
# looked at member→member, member→root, [dev-dependencies], [target.'cfg(…)'.dependencies]
# or the multi-line `[dependencies.x]` form, and the `crates/`-scoped globs never reached
# benchmarks/Cargo.toml (which shipped at 0.3.6). One parser now answers list/check/sync so
# the writer and the gate cannot disagree, and it exits 2 — never a verdict — when its line
# parser and tomllib disagree about a pin. tomllib is Python 3.11+; without it the audit
# cannot cross-check itself, so that is a skip, not a pass.
echo "==> Workspace version-pin edge gate (Cargo.toml intra-workspace pins)"
if python3 -c 'import tomllib' 2>/dev/null; then
  python3 "${SCRIPT_DIR}/workspace-version-edges.py" --check "$(cd "${SCRIPT_DIR}/.." && pwd)"
  echo "==> Workspace version-edge fixture"
  "${SCRIPT_DIR}/test-workspace-version-edges.sh"
elif [[ "${CSM_VALIDATION_REQUIRED}" == "true" ]]; then
  echo "ERROR: python3 tomllib (3.11+) is required by the validation gate; use python3.11+." >&2
  exit 1
else
  echo "SKIPPED — NOT A PASS: python3 has no tomllib; intra-workspace version pins went unchecked"
fi

# LLM context drift gate (#828): docs/architecture/context.yaml carried stale
# counters for months (skills 19/33, ADRs 16/95, active_wave 11/33, tests
# 134/1044) and two dead key-file paths, and nothing read the file. The
# validator now compares every counter and path against live sources; the
# fixture proves each failure direction. PyYAML is the one dependency
# (apt python3-yaml ships it in the CI lint job).
echo "==> LLM context drift gate (docs/architecture/context.yaml)"
_CSM_REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
if python3 -c 'import yaml' 2>/dev/null; then
  python3 "${_CSM_REPO_ROOT}/tests/validate_llm_context.py" --repo-root "${_CSM_REPO_ROOT}"
elif [[ "${CSM_VALIDATION_REQUIRED}" == "true" ]]; then
  echo "ERROR: PyYAML is required by the validation gate; install python3-yaml." >&2
  exit 1
else
  echo "SKIPPED — NOT A PASS: python3 has no PyYAML (apt: python3-yaml); context.yaml went unchecked"
fi
echo "==> LLM context drift gate fixture"
"${SCRIPT_DIR}/test-validate-llm-context.sh"

# ADR Registry consistency check (ADR-0076).  Keep one implementation of the
# registry↔disk policy; the standalone checker is also required by AGENTS.md.
echo "==> ADR Registry consistency check"
if [[ -x "${SCRIPT_DIR}/check-adr-parity.sh" ]]; then
  "${SCRIPT_DIR}/check-adr-parity.sh"
  echo "==> ADR parity gate fixture"
  "${SCRIPT_DIR}/test-check-adr-parity.sh"
else
  echo "Error: scripts/check-adr-parity.sh missing or not executable" >&2
  exit 1
fi

# GOAP status is a read-only operator view, but it is still a gate input for
# agents.  Keep inline comments and historical prose from changing the action
# counts reported by the orchestrator.
if [[ -x "${SCRIPT_DIR}/test-goap-orchestrator-status.sh" ]]; then
  echo "==> GOAP status parser fixture"
  "${SCRIPT_DIR}/test-goap-orchestrator-status.sh"
else
  echo "Error: scripts/test-goap-orchestrator-status.sh missing or not executable" >&2
  exit 1
fi

# Plan archive manifest completeness (issue #831). plans/ARCHIVE_MANIFEST.md is the
# only index of plans/.archive/, and `scripts/plans-manager.sh archive adr` moves ADRs
# into that directory without touching the manifest — so the gate had a real drift
# mechanism and no reader: 55 archived ADRs plus 49 handoffs went unlisted while
# `plan_archive_manifest_valid` reported true on prose. Bidirectional, so a stale row
# fails as loudly as an unlisted file.
if [[ -x "${SCRIPT_DIR}/check-archive-manifest.sh" ]]; then
  echo "==> Plan archive manifest completeness"
  "${SCRIPT_DIR}/check-archive-manifest.sh"
else
  echo "Error: scripts/check-archive-manifest.sh missing or not executable"
  exit 1
fi

echo "==> GitHub Actions workflow semantic validation"
"${SCRIPT_DIR}/check-actionlint.sh"
echo "==> actionlint dependency fixture"
"${SCRIPT_DIR}/test-check-actionlint.sh"
# GitHub Actions workflow YAML validation (issue #841). Fail-closed on the
# script's presence, and the fixture runs right after it: the gate separates
# parse failures from error-level lint findings from advisories, and that
# classifier is only trustworthy while it is being contradicted by a test.
# A box with neither yamllint nor python3+PyYAML is reported by the subject as
# "SKIPPED — NOT A PASS" and exits 0 — the skip is loud, never a green check.
if [[ -x "${SCRIPT_DIR}/validate-workflows.sh" ]]; then
  echo "==> GitHub Actions workflow YAML validation (fail-closed)"
  "${SCRIPT_DIR}/validate-workflows.sh" --check
  echo "==> Workflow YAML gate fixture"
  "${SCRIPT_DIR}/test-validate-workflows.sh"
else
  echo "Error: scripts/validate-workflows.sh missing or not executable"
  exit 1
fi

# Nix devShell host-helper audit (issue #949). The leak is a *shape*, not a runner:
# scripts/run-with-libs.sh exports LD_LIBRARY_PATH=<nix-store libs> and everything its
# command spawns inherits it, including the host `git` that cargo-deny uses for the
# RustSec advisory-db, which then dies in git-remote-https on GLIBC_ABI_DT_X86_64_PLT.
# The first diagnosis blamed the Ubuntu-26 runner image and pinned two jobs; the pinned
# runner failed the identical line and the unpinned `Cargo Deny` job passed it as the
# control, so this gate reads step bodies (PyYAML, same dependency as the YAML gates
# above) and never a `runs-on` value. The fixture runs right after it: a grep pattern
# with one stray paren reported "no unscrubbed host helper" for every input and passed
# the real tree, which is exactly the rot only a contradicted test catches.
if [[ -x "${SCRIPT_DIR}/check-nix-git-env.sh" ]]; then
  echo "==> Nix devShell host-helper audit (no unscrubbed git/curl under nix develop)"
  "${SCRIPT_DIR}/check-nix-git-env.sh" "$(cd "${SCRIPT_DIR}/.." && pwd)"
  echo "==> Nix environment fixture (both directions of every rule)"
  "${SCRIPT_DIR}/test-check-nix-git-env.sh"
else
  echo "Error: scripts/check-nix-git-env.sh missing or not executable" >&2
  exit 1
fi

# GitHub Actions SHA validation (optional - only if requested)
# Note: Disabled by default as existing workflows use version tags
# To enable: export CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true
if [[ -x "${SCRIPT_DIR}/validate-github-actions-shas.sh" ]] && [[ "${CSM_VALIDATE_GITHUB_ACTIONS_SHAS:-}" == "true" ]]; then
  echo "==> GitHub Actions SHA validation"
  "${SCRIPT_DIR}/validate-github-actions-shas.sh" --offline
else
  echo "skip: GitHub Actions SHA validation (use CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true to enable)"
fi

# Quality-gates regression fixture (issue #849). scripts/quality-gates.sh is not
# run by CI at all (`grep -rn nextest .github/workflows/` is empty), so its
# nextest branch was a machine-dependent trap: `cargo nextest run --quiet` dies
# in argument parsing with RC=2 before a single test compiles, and only boxes
# that happen to have cargo-nextest hit it. The fixture asserts the argv shape
# and replays it against the real binary, which is the only way this stays fixed
# without adding a nextest job to CI.
if [[ -x "${SCRIPT_DIR}/test-quality-gates.sh" ]]; then
  echo "==> Quality-gates fixture (nextest argv accepted, output contract)"
  "${SCRIPT_DIR}/test-quality-gates.sh"
else
  echo "Error: scripts/test-quality-gates.sh missing or not executable"
  exit 1
fi

# GOAP queue vs issue tracker reconciliation (issue #851). plans/ACTIONS.md and
# plans/GOAP_STATE.md had no reader at all: PR #836 merged as 2f26214 with
# `Fixes #824`, GitHub auto-closed #824, and on that same commit
# cover_sigterm_in_server_shutdown stayed queued with `status: in_progress` while
# `git show 2f26214:src/shutdown.rs | grep -c sigterm` returned 3. The gate is
# fail-closed on its own inputs and needs exactly one `gh issue list` call; its
# fixture runs here too so a neutered check cannot rot into a pass-for-the-wrong-
# reason (the orphaned-fixture class fixed by #846).
if [[ -x "${SCRIPT_DIR}/check-goap-queue-issues.sh" ]]; then
  echo "==> GOAP queue vs issue tracker reconciliation"
  "${SCRIPT_DIR}/check-goap-queue-issues.sh"
  echo "==> GOAP queue gate fixture (both directions of every check)"
  "${SCRIPT_DIR}/test-goap-queue-issues.sh"
else
  echo "Error: scripts/check-goap-queue-issues.sh missing or not executable"
  exit 1
fi
# RUSTSEC ignore parity between the reviewed deny.toml set and what cargo-audit reads (issue #903)
if [[ -x "${SCRIPT_DIR}/check-audit-ignore-parity.sh" ]]; then
  echo "==> RUSTSEC ignore parity (deny.toml vs .cargo/audit.toml, fail-closed)"
  "${SCRIPT_DIR}/check-audit-ignore-parity.sh"
  echo "==> RUSTSEC parity gate fixture (both directions)"
  "${SCRIPT_DIR}/test-check-audit-ignore-parity.sh"
else
  echo "Error: scripts/check-audit-ignore-parity.sh missing or not executable"
  exit 1
fi
# The zip advisory ignores are only safe while `zip` is locked as a build-dependency
# and csm-duckdb still enables `duckdb/bundled` (issue #966). `cargo deny` can see
# neither fact, and GHSA-m665-v7pm-7grj had no RustSec id at filing time. The gate
# also requires the GHSA record and its re-check trigger, so a future removal of
# either safety fact cannot leave the ignore silently justified.
if [[ -x "${SCRIPT_DIR}/check-bundled-ignore-reason.sh" ]]; then
  echo "==> Bundled advisory-ignore reason (zip lock + duckdb/bundled, fail-closed)"
  "${SCRIPT_DIR}/check-bundled-ignore-reason.sh"
  echo "==> Bundled advisory-ignore fixture (all failure directions)"
  "${SCRIPT_DIR}/test-check-bundled-ignore-reason.sh"
else
  echo "Error: scripts/check-bundled-ignore-reason.sh missing or not executable" >&2
  exit 1
fi

echo "Validation complete."
