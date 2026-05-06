#!/usr/bin/env bash
# Pre-release validation script
# Run before every git tag / release to verify README commands work
# Usage: ./scripts/pre-release-validate.sh [--skip-bench] [--dry-run]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"
cd "${PROJECT_ROOT}"

SKIP_BENCH=false
DRY_RUN=false
VERSION=""
FAILED=0
PASSED=0

for arg in "$@"; do
    case $arg in
        --skip-bench) SKIP_BENCH=true ;;
        --dry-run) DRY_RUN=true ;;
        --version=*) VERSION="${arg#*=}" ;;
    esac
done

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASSED++))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAILED++))
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

check_command() {
    if command -v "$1" &> /dev/null; then
        pass "$1 is installed"
        return 0
    else
        fail "$1 is not installed"
        return 1
    fi
}

# ============================================================================
# SECTION 1: Prerequisites
# ============================================================================
section "1. Prerequisites"

check_command cargo || true
check_command rustup || true
check_command git || true

if [ -n "$VERSION" ]; then
    echo "Target version: $VERSION"
fi

# ============================================================================
# SECTION 2: Build Verification
# ============================================================================
section "2. Build Verification"

echo "Building release binary..."
if cargo build --release --bin csm 2>&1 | tail -3; then
    pass "Release binary built"
else
    fail "Release binary build failed"
fi

# ============================================================================
# SECTION 3: README CLI Commands Verification
# ============================================================================
section "3. README CLI Commands Verification"

CSM="./target/release/csm"
TEST_DB="/tmp/csm_test_$$.db"
TEST_JSON="/tmp/csm_test_$$.json"

cleanup() {
    rm -f "${TEST_DB}" "${TEST_JSON}" backup.json /tmp/csm_test_*.db /tmp/csm_test_*.json 2>/dev/null || true
}
trap cleanup EXIT

# 3.1 Version command
echo "Testing: csm version"
if $CSM version 2>&1 | grep -q "csm"; then
    pass "csm version works"
else
    fail "csm version failed"
fi

# 3.2 Inject command
echo "Testing: csm inject my-concept --database memory.db"
if $CSM inject test-concept --database "${TEST_DB}" 2>&1 | grep -qE "(injected|updated)"; then
    pass "csm inject works"
else
    fail "csm inject failed"
fi

# 3.3 Probe command
echo "Testing: csm probe my-concept -k 10 --database memory.db"
if $CSM probe test-concept -k 10 --database "${TEST_DB}" 2>&1 | grep -qE "(Found|results|similar concepts)"; then
    pass "csm probe works"
else
    fail "csm probe failed"
fi

# 3.4 Associate command (inject source first)
echo "Testing: csm associate source-concept target-concept --strength 0.8"
$CSM inject source-concept --database "${TEST_DB}" 2>&1 > /dev/null
$CSM inject target-concept --database "${TEST_DB}" 2>&1 > /dev/null
if $CSM associate source-concept target-concept --strength 0.8 --database "${TEST_DB}" 2>&1 | grep -qE "(association|created)"; then
    pass "csm associate works"
else
    fail "csm associate failed"
fi

# 3.5 Export command
echo "Testing: csm export --output backup.json"
EXPORT_OUTPUT=$($CSM export --output "${TEST_JSON}" --database "${TEST_DB}" 2>&1 || echo "FAILED")
if echo "${EXPORT_OUTPUT}" | grep -qE "(exported|Exporting)"; then
    pass "csm export works"
else
    fail "csm export failed: ${EXPORT_OUTPUT}"
fi

# 3.6 Import command
echo "Testing: csm import backup.json --merge"
IMPORT_OUTPUT=$($CSM import "${TEST_JSON}" --merge --database "${TEST_DB}" 2>&1 || echo "FAILED")
if echo "${IMPORT_OUTPUT}" | grep -qE "(imported|Importing|concepts)"; then
    pass "csm import works"
else
    fail "csm import failed: ${IMPORT_OUTPUT}"
fi

# 3.7 Completions command
echo "Testing: csm completions bash"
if $CSM completions bash 2>&1 | head -5 | grep -q "_csm"; then
    pass "csm completions works"
else
    fail "csm completions failed"
fi

# ============================================================================
# SECTION 4: Development Gates (from README)
# ============================================================================
section "4. Development Gates"

# 4.1 cargo check
echo "Testing: cargo check --quiet"
if cargo check --quiet 2>&1; then
    pass "cargo check passed"
else
    fail "cargo check failed"
fi

# 4.2 cargo test
echo "Testing: cargo test --all-features --quiet"
if cargo test --all-features --quiet 2>&1 | tail -5; then
    pass "cargo test passed"
else
    fail "cargo test failed"
fi

# 4.3 cargo fmt --check
echo "Testing: cargo fmt --check --quiet"
if cargo fmt --check --quiet 2>&1; then
    pass "cargo fmt --check passed"
else
    fail "cargo fmt --check failed"
fi

# 4.4 cargo clippy
echo "Testing: cargo clippy --quiet -- -D warnings"
if cargo clippy --quiet -- -D warnings 2>&1; then
    pass "cargo clippy passed"
else
    fail "cargo clippy failed"
fi

# ============================================================================
# SECTION 5: LOC Policy
# ============================================================================
section "5. LOC Policy (<= 500 lines per file)"

LOC_FAILED=0
for file in $(find src -name '*.rs'); do
    loc=$(wc -l < "${file}")
    if [ "${loc}" -gt 500 ]; then
        fail "${file} has ${loc} lines (max 500)"
        ((LOC_FAILED++))
    else
        pass "${file} (${loc} LOC)"
    fi
done

if [ "${LOC_FAILED}" -eq 0 ]; then
    pass "All source files under 500 LOC"
fi

# ============================================================================
# SECTION 6: WASM Build
# ============================================================================
section "6. WASM Build"

WASM_TARGET="wasm32-unknown-unknown"
if rustup target list --installed | grep -q "^${WASM_TARGET}$"; then
    echo "Testing: cargo check --target wasm32-unknown-unknown --features wasm"
    if cargo check --target "${WASM_TARGET}" --features wasm 2>&1 | tail -3; then
        pass "WASM build passed"
    else
        fail "WASM build failed"
    fi
else
    warn "WASM target not installed, skipping"
fi

# ============================================================================
# SECTION 7: WASM Size Gate
# ============================================================================
section "7. WASM Size Gate"

if [ -x scripts/wasm_size_gate.sh ]; then
    if scripts/wasm_size_gate.sh 2>&1 | tail -3; then
        pass "WASM size gate passed"
    else
        fail "WASM size gate failed"
    fi
else
    warn "wasm_size_gate.sh not found, skipping"
fi

# ============================================================================
# SECTION 8: Benchmark Gates
# ============================================================================
section "8. Benchmark Gates"

if [ "$SKIP_BENCH" = true ]; then
    warn "Skipping benchmarks (--skip-bench)"
elif [ -f "benches/benchmark.rs" ]; then
    echo "Testing: cargo bench --bench benchmark -- --baseline main"
    if timeout 180 cargo bench --bench benchmark -- --baseline main 2>&1 | tail -20; then
        pass "Benchmarks passed"
    else
        warn "Benchmarks failed or no baseline exists (run with --save-baseline main first)"
    fi
else
    warn "No benchmark file found, skipping"
fi

# ============================================================================
# SECTION 9: Version Sync Script
# ============================================================================
section "9. Version Sync Script"

if [ -x scripts/sync-version.sh ]; then
    if [ -n "$VERSION" ] && [ "$DRY_RUN" = false ]; then
        echo "Running: ./scripts/sync-version.sh $VERSION"
        ./scripts/sync-version.sh "$VERSION"
        pass "Version synced to $VERSION"
    else
        echo "Usage: ./scripts/sync-version.sh <version>"
        pass "Version sync script available"
    fi
else
    fail "scripts/sync-version.sh not found"
fi

# ============================================================================
# SECTION 10: Validation Script
# ============================================================================
section "10. Full Validation Script"

if [ -x scripts/validate.sh ]; then
    if scripts/validate.sh 2>&1 | tail -30; then
        pass "validate.sh passed"
    else
        fail "validate.sh failed"
    fi
else
    fail "scripts/validate.sh not found"
fi

# ============================================================================
# Summary
# ============================================================================
section "Summary"

TOTAL=$((PASSED + FAILED))
echo ""
echo "Passed: ${PASSED}/${TOTAL}"
echo "Failed: ${FAILED}/${TOTAL}"
echo ""

if [ "${FAILED}" -gt 0 ]; then
    echo -e "${RED}❌ Pre-release validation FAILED${NC}"
    echo "Fix the issues above before creating a release."
    exit 1
else
    echo -e "${GREEN}✅ Pre-release validation PASSED${NC}"
    echo ""
    echo "Next steps for release:"
    echo "  1. ./scripts/sync-version.sh <version>  # e.g., 0.2.0"
    echo "  2. git add -A && git commit -m 'release: v<version>'"
    echo "  3. git tag -a v<version> -m 'Release <version>'"
    echo "  4. git push origin main v<version>"
    exit 0
fi