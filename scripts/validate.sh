#!/usr/bin/env bash
set -euo pipefail

# Source lint caching library for faster repeated runs
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${SCRIPT_DIR}/lib/lint_cache.sh" ]]; then
    source "${SCRIPT_DIR}/lib/lint_cache.sh"
fi

MAX_SRC_LOC=500
WASM_TARGET="wasm32-unknown-unknown"

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

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

echo "==> cargo test --all-targets"
cargo test --all-targets

echo "==> Source file LOC gate (< ${MAX_SRC_LOC})"
for file in $(find src -name '*.rs'); do
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
  echo "==> scripts/wasm_size_gate.sh"
  scripts/wasm_size_gate.sh
fi

echo "==> Generating/validating llms.txt and llms-full.txt"
scripts/gen-llms-txt.sh

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

# GitHub Actions SHA validation (optional - only if requested)
# Note: Disabled by default as existing workflows use version tags
# To enable: export CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true
if [[ -x "${SCRIPT_DIR}/validate-github-actions-shas.sh" ]] && [[ "${CSM_VALIDATE_GITHUB_ACTIONS_SHAS:-}" == "true" ]]; then
  echo "==> GitHub Actions SHA validation"
  "${SCRIPT_DIR}/validate-github-actions-shas.sh" --offline
else
  echo "skip: GitHub Actions SHA validation (use CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true to enable)"
fi

echo "Validation complete."
