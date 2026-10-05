#!/usr/bin/env bash
set -euo pipefail

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

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

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
for file in $(find src crates -name '*.rs' -not -path '*/target/*'); do
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
done

if rustup target list --installed | grep -q "^${WASM_TARGET}\$"; then
  echo "==> cargo check --target ${WASM_TARGET} --features wasm"
  cargo check --target "${WASM_TARGET}" --features wasm
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
for fixture in test-llms-sync.sh test-version-sync.sh; do
  if [[ -x "${SCRIPT_DIR}/${fixture}" ]]; then
    echo "==> Fixture: scripts/${fixture}"
    "${SCRIPT_DIR}/${fixture}"
  else
    echo "Error: scripts/${fixture} missing or not executable"
    exit 1
  fi
done

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
  cargo build --release --bin csm
  mkdir -p cli-npm/bin
  cp target/release/csm cli-npm/bin/csm-linux-x64
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

# ShellCheck for all shell scripts (optional - only if installed)
# Note: Disabled due to shellcheck crash on scripts with path references
# Shellcheck bug: https://github.com/koalaman/shellcheck/issues/XXXX
# Re-enable when shellcheck is fixed or when we have a workaround
if command -v shellcheck >/dev/null 2>&1 && [[ "${CSM_ENABLE_SHELLCHECK:-}" == "true" ]]; then
  echo "==> ShellCheck (severity=error)"
  SHELL_SCRIPTS=$(find scripts -name '*.sh' -type f)
  if [[ -n "${SHELL_SCRIPTS}" ]]; then
    shellcheck --severity=error "${SHELL_SCRIPTS}"
    echo "ok: all shell scripts pass shellcheck"
  else
    echo "skip: no shell scripts found"
  fi
else
  echo "skip: shellcheck disabled (crashes on path references)"
  echo "      To enable: export CSM_ENABLE_SHELLCHECK=true"
fi

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

# ADR Registry consistency check (ADR-0076)
echo "==> ADR Registry consistency check"
ADR_REGISTRY="plans/ADR_REGISTRY.md"
ADR_DIR="plans/adr"
if [[ -f "${ADR_REGISTRY}" ]]; then
  # Extract ADR numbers from registry table
  REGISTRY_ADRS=$(grep -oE '\| [0-9]{4} \|' "${ADR_REGISTRY}" | sed 's/|//g' | tr -d ' ' | sort -u | grep -E '^[0-9]{4}$')
  # Check for missing files
  MISSING_COUNT=0
  for adr_num in $REGISTRY_ADRS; do
    # Skip superseded ADR-0003
    if [[ "$adr_num" == "0003" ]]; then
      continue
    fi
    # Find matching file (allow any suffix after number)
    ADR_FILE=$(find "${ADR_DIR}" -name "${adr_num}-*.md" -type f 2>/dev/null | head -1)
    if [[ -z "${ADR_FILE}" ]]; then
      echo "Missing ADR file: ${adr_num}"
      MISSING_COUNT=$((MISSING_COUNT + 1))
    fi
  done
  if [[ $MISSING_COUNT -gt 0 ]]; then
    echo "Error: ${MISSING_COUNT} ADR files missing from ${ADR_DIR}"
    exit 1
  fi
  # Count ADR files and report
  ADR_FILE_COUNT=$(find "${ADR_DIR}" -name '*.md' -type f | wc -l)
  echo "ok: ${ADR_FILE_COUNT} ADR files in ${ADR_DIR}"
else
  echo "skip: ${ADR_REGISTRY} not found"
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

echo "Validation complete."
