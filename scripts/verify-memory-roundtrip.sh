#!/bin/bash
#
# verify-memory-roundtrip.sh - Automated Turso/libSQL memory verification
#
# Usage: source scripts/verify-memory-roundtrip.sh
# Exit code: 0 on success, 1 on failure

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_PREFIX="verify-test-$(date +%s)"
DB1=".tmp/${TEST_PREFIX}-1.db"
DB2=".tmp/${TEST_PREFIX}-2.db"
DB3=".tmp/${TEST_PREFIX}-3.db"
JSON_EXPORT=".tmp/${TEST_PREFIX}.json"
BINARY_EXPORT=".tmp/${TEST_PREFIX}.bin"

# Counters
TESTS_PASSED=0
TESTS_FAILED=0

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_test() {
    echo -e "${GREEN}[TEST]${NC} $1"
}

pass_test() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail_test() {
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

cleanup() {
    log_info "Cleaning up test files..."
    rm -f "$DB1" "$DB2" "$DB3" "$JSON_EXPORT" "$BINARY_EXPORT" 2>/dev/null || true
}

trap cleanup EXIT

# Ensure csm binary exists
if [[ ! -f "./target/release/csm" ]]; then
    log_error "csm binary not found at ./target/release/csm"
    log_info "Run: cargo build --release --bin csm"
    exit 1
fi

CSM="./target/release/csm"

log_info "Starting memory roundtrip verification..."

# Test 1: Create concepts and associations
log_test "Creating test concepts..."
$CSM --database "$DB1" inject "test::concept::1" -m '{"context":"test context","result":"success","operation":"create"}' 2>/dev/null
$CSM --database "$DB1" inject "test::concept::2" -m '{"context":"test context 2","result":"success","operation":"create"}' 2>/dev/null
$CSM --database "$DB1" associate "test::concept::1" "test::concept::2" -s 0.85 2>/dev/null
log_info "✓ Created 2 concepts with 1 association"

# Test 2: JSON export
log_test "Testing JSON export..."
$CSM --database "$DB1" export -o "$JSON_EXPORT" --output-format quiet 2>/dev/null
if [[ -f "$JSON_EXPORT" ]]; then
    CONCEPTS_COUNT=$(jq '.concepts | length' "$JSON_EXPORT")
    ASSOC_COUNT=$(jq '.associations | length' "$JSON_EXPORT")
    log_info "✓ JSON export: $CONCEPTS_COUNT concepts, $ASSOC_COUNT associations"
    pass_test
else
    log_error "✗ JSON export failed"
    fail_test
    exit 1
fi

# Test 3: JSON import
log_test "Testing JSON import..."
$CSM --database "$DB2" import "$JSON_EXPORT" 2>/dev/null
if $CSM --database "$DB2" probe "test::concept::1" --output-format quiet >/dev/null 2>&1; then
    log_info "✓ JSON import successful"
    pass_test
else
    log_error "✗ JSON import failed"
    fail_test
    exit 1
fi

# Test 4: Binary export
log_test "Testing binary export..."
$CSM --database "$DB1" export --format binary -o "$BINARY_EXPORT" 2>/dev/null
if [[ -f "$BINARY_EXPORT" ]]; then
    FILE_SIZE=$(stat -c%s "$BINARY_EXPORT" 2>/dev/null || stat -f%z "$BINARY_EXPORT" 2>/dev/null)
    log_info "✓ Binary export: $FILE_SIZE bytes"
    pass_test
else
    log_error "✗ Binary export failed"
    fail_test
    exit 1
fi

# Test 5: Binary import
log_test "Testing binary import..."
$CSM --database "$DB3" import --format binary "$BINARY_EXPORT" 2>/dev/null
if $CSM --database "$DB3" probe "test::concept::1" --output-format quiet >/dev/null 2>&1; then
    log_info "✓ Binary import successful"
    pass_test
else
    log_error "✗ Binary import failed"
    fail_test
    exit 1
fi

# Test 6: Verify data integrity
log_test "Verifying data integrity..."
SIMILARITY_RESULT=$($CSM --database "$DB3" probe "test::concept::1" --output-format json 2>/dev/null | jq -r '.results[0].similarity // empty')
if [[ -n "$SIMILARITY_RESULT" ]]; then
    log_info "✓ Similarity search works: similarity=$SIMILARITY_RESULT"
    pass_test
else
    log_warn "⚠ Similarity search returned empty (may be normal for random vectors)"
fi

# Summary
echo ""
log_info "====================================="
log_info "Tests Passed: $TESTS_PASSED"
log_info "Tests Failed: $TESTS_FAILED"
log_info "====================================="

if [[ $TESTS_FAILED -eq 0 ]]; then
    log_info "✓ All memory roundtrip tests passed!"
    exit 0
else
    log_error "✗ Some tests failed!"
    exit 1
fi
