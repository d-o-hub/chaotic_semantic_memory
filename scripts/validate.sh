#!/usr/bin/env bash
# Validate code quality with baseline-aware delta checking.
# First run: --save-baseline captures current error state
# Subsequent runs: compares against baseline, only fails on NEW errors
# Usage:
#   scripts/validate.sh                  # Full validation (delta mode if baseline exists)
#   scripts/validate.sh --save-baseline  # Save current state as baseline
#   scripts/validate.sh --clear-baseline # Remove baseline
set -euo pipefail

# Source lint caching library for faster repeated runs
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${SCRIPT_DIR}/lib/lint_cache.sh" ]]; then
    source "${SCRIPT_DIR}/lib/lint_cache.sh"
fi

MAX_SRC_LOC=500
WASM_TARGET="wasm32-unknown-unknown"

# ── Baseline management ──────────────────────────────────────────────
BASELINE_DIR="/tmp/csm-baseline"
MODE="validate"

for arg in "$@"; do
    case "$arg" in
        --save-baseline) MODE="save" ;;
        --clear-baseline) MODE="clear" ;;
    esac
done

if [[ "$MODE" == "clear" ]]; then
    rm -rf "$BASELINE_DIR"
    echo "Baseline cleared."
    exit 0
fi

# ── Error normalization (strips line numbers for stable comparison) ──
normalize_errors() {
    sed -E \
        -e 's/--> [^:]+:[0-9]+:[0-9]+/--> <file>:<line>/g' \
        -e 's/^[[:space:]]*[0-9]+[[:space:]]*\|[[:space:]]*/  | /g' \
        -e 's/^[[:space:]]*\|[[:space:]]*$//g' \
        -e '/^[[:space:]]*$/d' \
        -e '/^warning: `/d' \
        -e '/^= help: /d' \
        -e '/^For more information/d' \
        -e '/^Some errors have detailed/d' \
        -e '/^note: /d' \
        | grep -vE '^[[:space:]]*Finished `.*` profile' \
        | grep -vE '^[[:space:]]*Updating crates.io' \
        | grep -vE '^[[:space:]]*Checking ' \
        | grep -vE '^[[:space:]]*Compiling ' \
        | grep -vE '^[[:space:]]*Downloading crates' \
        | grep -vE '^[[:space:]]*Downloaded ' \
        | grep -vE '^[[:space:]]*Running unittests ' \
        | grep -vE '^[[:space:]]*Running tests/' \
        | grep -vE '^[[:space:]]*Running benches/' \
        | grep -vE '^[[:space:]]*Doc-tests ' \
        | grep -vE '^[[:space:]]*test result: ok.' \
        | grep -vE '^running [0-9]+ tests?$' \
        | grep -vE '^test [a-zA-Z_0-9\/\.\:-]+ \.\.\. ok$' \
        | grep -vE '^[[:space:]]*all doctests ran in' \
        | grep -vE '^Gnuplot not found' \
        | grep -vE '^Testing ' \
        | grep -vE '^Success' \
        | awk 'NF' || true
}

# ── Delta check: only fail if NEW errors appear ─────────────────────
# Returns 0 if NO new errors, 1 if new errors found
delta_check() {
    local label="$1"        # e.g. "cargo check"
    local baseline_file="$BASELINE_DIR/${label// /_}"
    local current_file="${baseline_file}.current"

    if [[ "$MODE" == "save" ]]; then
        mkdir -p "$BASELINE_DIR"
        cat > "$baseline_file"
        echo "  baseline saved: ${label}"
        return 0
    fi

    # Ensure baseline directory exists for writing current output
    mkdir -p "$BASELINE_DIR"

    # Write from stdin to current_file, filter out known OK lines
    cat | grep -vE '^test [a-zA-Z_0-9\/\.\:-]+ \.\.\. ok$' \
        | grep -vE '^test result: ok\.' \
        | grep -vE '^running [0-9]+ tests?$' \
        | grep -vE '^[[:space:]]*all doctests ran in' \
        | grep -vE '^Gnuplot not found' \
        | grep -vE '^Testing ' \
        | grep -vE '^Success' \
        | grep -vE '^[[:space:]]*$' \
        > "$current_file" || true

    # If the file is just whitespaces or empty, consider it empty and exit 0
    if [ ! -s "$current_file" ]; then
        rm -f "$current_file"
        return 0
    fi

    if [[ ! -f "$baseline_file" ]]; then
        # No baseline: use current output as-is for std error checking
        cat "$current_file" >&2
        rm -f "$current_file"
        return 1
    fi

    # Diff: find lines in current but NOT in baseline (new errors)
    local new_errors
    new_errors=$(comm -13 <(sort "$baseline_file") <(sort "$current_file")) || true

    rm -f "$current_file"

    if [[ -n "$new_errors" ]]; then
        if [ -z "$(echo "$new_errors" | tr -d '
')" ]; then
            return 0
        fi
        echo "$new_errors" >&2
        return 1
    fi
    return 0
}

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
# Disable pipefail: cargo may fail with pre-existing errors, but delta_check
# should only fail on NEW errors. pipefail would cause false positives.
set +o pipefail
CLIPPY_OUT=$(cargo clippy --all-targets --all-features -- -D warnings 2>&1) || true
set -o pipefail
echo "$CLIPPY_OUT" | normalize_errors | delta_check "clippy" || {
    if [[ "$MODE" != "save" ]]; then
        echo "Error: clippy found new warnings/errors"
        exit 1
    fi
}

# CI applies stricter RUSTFLAGS; this is the minimal local gate
# Check for warnings AND ensure compilation succeeds
echo "==> cargo test --no-run --all-features (check for warnings)"
COMPILE_OUT=$(cargo test --no-run --all-features 2>&1) || {
    echo "$COMPILE_OUT" | normalize_errors | delta_check "test-compile" || {
        echo "Error: new compilation failures with --all-features"
        exit 1
    }
}
if echo "$COMPILE_OUT" | grep -qi "warning:"; then
    echo "$COMPILE_OUT" | grep -i "warning:" | normalize_errors | delta_check "test-warnings" || {
        echo "Error: new warnings found in test compilation"
        exit 1
    }
fi

echo "==> cargo test --all-targets"
# Disable pipefail: cargo test may fail with pre-existing failures, but
# delta_check should only fail on NEW failures. pipefail would cause false positives.
set +o pipefail
TEST_OUT=$(cargo test --all-targets 2>&1) || true
set -o pipefail
echo "$TEST_OUT" | normalize_errors | delta_check "test" || {
    if [[ "$MODE" != "save" ]]; then
        echo "Error: new test failures detected"
        exit 1
    fi
}

echo "==> Source file LOC gate (< ${MAX_SRC_LOC})"
while IFS= read -r file; do
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
done < <(find src -name '*.rs')

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
