#!/usr/bin/env bash
# scripts/quality-gates.sh
# Full quality gate with auto-detection for multiple languages.
# Usage: ./scripts/quality-gates.sh [--fix]
# Exit 0 = success, Exit 1 = errors.

set +e
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# --- Configuration ---
readonly GIT_EXCLUDE="./.git/*"
readonly MAX_LINES_PER_SOURCE_FILE=500

# --- Parse arguments ---
FIX=false
for arg in "$@"; do
  case $arg in
    --fix) FIX=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

# --- Colors ---
if [[ -t 1 ]] && [[ "${FORCE_COLOR:-}" != "0" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  NC=''
fi

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; FAILED=1; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
info() { echo -e "${BLUE}[INFO]${NC} $1"; }

FAILED=0
DETECTED_LANGUAGES=()

# Determine context
GITHUB_EVENT="${GITHUB_EVENT:-${GITHUB_EVENT_NAME:-}}"
GITHUB_REF="${GITHUB_REF:-}"

printf "Running quality gate...\n\n"

# ============================================================
# 1. LANGUAGE DETECTION
# ============================================================
info "Detecting project languages..."

if [[ -f "Cargo.toml" ]]; then
  printf " ${GREEN}✓${NC} Rust (Cargo.toml)\n"
  DETECTED_LANGUAGES+=("rust")
fi

if [[ -f "package.json" ]] || [[ -f "wasm/package.json" ]] || [[ -f "cli-npm/package.json" ]]; then
  printf " ${GREEN}✓${NC} TypeScript/JavaScript/WASM (package.json detected)\n"
  DETECTED_LANGUAGES+=("typescript")
fi

if find . -name "*.sh" -not -path "$GIT_EXCLUDE" -not -path "./target/*" -print -quit 2>/dev/null | grep -q .; then
  printf " ${GREEN}✓${NC} Shell scripts detected\n"
  DETECTED_LANGUAGES+=("shell")
fi

if find . -name "*.md" -not -path "$GIT_EXCLUDE" -not -path "./target/*" -print -quit 2>/dev/null | grep -q .; then
  printf " ${GREEN}✓${NC} Markdown files detected\n"
  DETECTED_LANGUAGES+=("markdown")
fi

if [[ ${#DETECTED_LANGUAGES[@]} -eq 0 ]]; then
  warn "No recognized project files found."
fi
printf "\n"

# ============================================================
# 2. LOC LIMITS
# ============================================================
info "Enforcing LOC limits (max ${MAX_LINES_PER_SOURCE_FILE} lines per file)..."
LOC_VIOLATIONS=0
while IFS= read -r file; do
  lines=$(wc -l < "$file" 2>/dev/null || echo 0)
  if [[ "$lines" -gt "$MAX_LINES_PER_SOURCE_FILE" ]]; then
    warn " $file: $lines lines (max $MAX_LINES_PER_SOURCE_FILE)"
    LOC_VIOLATIONS=$((LOC_VIOLATIONS + 1))
  fi
done < <(find src crates -name "*.rs" -not -path "*/target/*" -not -path "./.git/*" -type f 2>/dev/null)

if [[ $LOC_VIOLATIONS -gt 0 ]]; then
  fail "LOC: $LOC_VIOLATIONS files exceed ${MAX_LINES_PER_SOURCE_FILE} lines"
else
  pass "LOC: All source files within limit"
fi
printf "\n"

# ============================================================
# 3. RUST CHECKS
# ============================================================
if [[ " ${DETECTED_LANGUAGES[*]} " =~ " rust " ]]; then
  info "Running Rust checks..."

  # Format
  if $FIX; then
    cargo fmt --all
    pass "Format: auto-fixed"
  else
    if ! OUTPUT=$(cargo fmt --all -- --check 2>&1); then
      fail "Format: run 'cargo fmt --all' to fix"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Format: OK"
    fi
  fi

  # Clippy
  if $FIX; then
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
    pass "Clippy: auto-fixed"
  else
    if ! OUTPUT=$(cargo clippy --all-targets --all-features -- -D warnings 2>&1); then
      fail "Clippy: fix lint errors above"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Clippy: OK"
    fi
  fi

  # Build
  if ! OUTPUT=$(cargo build --all-targets --quiet 2>&1); then
    fail "Build: failed"
    printf "%s\n" "$OUTPUT" >&2
  else
    pass "Build: OK"
  fi

  # Tests
  if command -v cargo-nextest &>/dev/null; then
    if ! OUTPUT=$(cargo nextest run --all-features --workspace --quiet 2>&1); then
      fail "Tests: failed"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Tests (nextest): OK"
    fi
  else
    if ! OUTPUT=$(cargo test --all-features --workspace --quiet 2>&1); then
      fail "Tests: failed"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Tests: OK"
    fi
  fi

  # Security audit
  if command -v cargo-audit &>/dev/null; then
    if ! OUTPUT=$(cargo audit --quiet 2>&1); then
      fail "Security audit: vulnerabilities found"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Audit: OK"
    fi
  else
    warn "cargo-audit not installed, skipping"
  fi

  # Supply chain
  if command -v cargo-deny &>/dev/null; then
    if ! OUTPUT=$(cargo deny check 2>&1); then
      fail "cargo-deny: violations found"
      printf "%s\n" "$OUTPUT" >&2
    else
      pass "Deny: OK"
    fi
  else
    warn "cargo-deny not installed, skipping"
  fi

  printf "\n"
fi

# ============================================================
# 4. SHELL CHECKS
# ============================================================
if [[ " ${DETECTED_LANGUAGES[*]} " =~ " shell " ]]; then
  info "Running Shell script checks..."
  if command -v shellcheck &>/dev/null; then
    TMP_SH_LIST=$(mktemp)
    find scripts -name "*.sh" -not -path "$GIT_EXCLUDE" -not -path "./target/*" -print0 2>/dev/null > "$TMP_SH_LIST" || true
    if [[ -s "$TMP_SH_LIST" ]]; then
      if ! xargs -0 shellcheck --severity=error < "$TMP_SH_LIST" 2>/dev/null; then
        fail "shellcheck failed"
      else
        pass "shellcheck: OK"
      fi
    fi
    rm -f -- "$TMP_SH_LIST"
  else
    warn "shellcheck not installed, skipping"
  fi
  printf "\n"
fi

# ============================================================
# 5. MARKDOWN CHECKS
# ============================================================
if [[ " ${DETECTED_LANGUAGES[*]} " =~ " markdown " ]]; then
  info "Running Markdown checks..."
  if command -v markdownlint-cli2 &>/dev/null; then
    if ! markdownlint-cli2 "**/*.md" "#node_modules" >/dev/null 2>&1; then
      warn "markdownlint-cli2: issues found (run 'markdownlint-cli2 \"**/*.md\"' locally)"
    else
      pass "markdownlint: OK"
    fi
  elif command -v markdownlint &>/dev/null; then
      if ! markdownlint "**/*.md" --ignore "node_modules" >/dev/null 2>&1; then
          warn "markdownlint: issues found"
      else
          pass "markdownlint: OK"
      fi
  else
    warn "markdownlint not installed, skipping"
  fi
  printf "\n"
fi

# ============================================================
# SUMMARY
# ============================================================
if [[ $FAILED -ne 0 ]]; then
  printf "${RED}─────────────────────────────────────────────────────────────────${NC}\n"
  printf "${RED}│ ✗ Quality Gate FAILED │${NC}\n"
  printf "${RED}─────────────────────────────────────────────────────────────────${NC}\n"
  exit 1
fi

printf "${GREEN}─────────────────────────────────────────────────────────────────${NC}\n"
printf "${GREEN}│ ✓ All Quality Gates PASSED │${NC}\n"
printf "${GREEN}─────────────────────────────────────────────────────────────────${NC}\n"
