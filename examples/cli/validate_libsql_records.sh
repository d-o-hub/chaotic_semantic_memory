#!/usr/bin/env bash
#
# validate_libsql_records.sh
# 
# Validates that CLI edge case examples work correctly with libsql persistence.
# This script verifies records are properly stored and retrievable from the database.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0

# Temporary database
DB_FILE=$(mktemp -t csm_validate_XXXXXX.db)
trap "rm -f '$DB_FILE'" EXIT

csm() {
    cargo run --quiet --bin csm -- --database "$DB_FILE" "$@"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

log_section() {
    echo -e "\n${YELLOW}=== $1 ===${NC}\n"
}

# ============================================================================
# Test 1: Basic CRUD with libsql persistence
# ============================================================================
test_basic_crud() {
    log_section "Test 1: Basic CRUD Operations"
    
    # Inject concept
    log_info "Injecting concept 'test_concept'"
    if csm inject test_concept > /dev/null 2>&1; then
        log_success "Concept injection successful"
    else
        log_error "Concept injection failed"
        return 1
    fi
    
    # Verify concept exists by probing
    log_info "Probing for 'test_concept'"
    if csm probe test_concept -k 1 > /dev/null 2>&1; then
        log_success "Concept retrieval successful"
    else
        log_error "Concept retrieval failed"
        return 1
    fi
    
    # Inject second concept and create association
    log_info "Creating second concept and association"
    csm inject related_concept > /dev/null 2>&1
    if csm associate test_concept related_concept -s 0.75 > /dev/null 2>&1; then
        log_success "Association creation successful"
    else
        log_error "Association creation failed"
        return 1
    fi
    
    # Export and verify structure
    log_info "Exporting database"
    EXPORT_OUTPUT=$(csm export -o - 2>/dev/null)
    if echo "$EXPORT_OUTPUT" | grep -q '"concepts"' && echo "$EXPORT_OUTPUT" | grep -q '"associations"'; then
        log_success "Export contains expected structure"
    else
        log_error "Export missing expected structure"
        return 1
    fi
    
    # Verify record count
    CONCEPT_COUNT=$(echo "$EXPORT_OUTPUT" | grep -o '"id"' | wc -l)
    log_info "Found $CONCEPT_COUNT concepts in export"
    if [ "$CONCEPT_COUNT" -eq 2 ]; then
        log_success "Correct concept count in database"
    else
        log_error "Expected 2 concepts, found $CONCEPT_COUNT"
        return 1
    fi
}

# ============================================================================
# Test 2: Edge case record validation
# ============================================================================
test_edge_cases() {
    log_section "Test 2: Edge Case Record Validation"
    
    # Test 2a: Concept with maximum valid ID (256 bytes)
    log_info "Testing maximum valid concept ID (256 bytes)"
    MAX_ID=$(python3 -c "print('a' * 256)")
    if csm inject "$MAX_ID" > /dev/null 2>&1; then
        log_success "256-byte ID accepted and stored"
        
        # Verify retrievable
        if csm probe "$MAX_ID" -k 1 > /dev/null 2>&1; then
            log_success "256-byte ID concept retrievable"
        else
            log_error "256-byte ID concept not retrievable"
        fi
    else
        log_error "256-byte ID should be accepted"
    fi
    
    # Test 2b: Concept with oversized ID (257 bytes) - should fail
    log_info "Testing oversized concept ID (257 bytes)"
    OVERSIZED_ID=$(python3 -c "print('b' * 257)")
    if ! csm inject "$OVERSIZED_ID" > /dev/null 2>&1; then
        log_success "257-byte ID correctly rejected"
    else
        log_error "257-byte ID should be rejected"
    fi
    
    # Test 2c: Empty ID - should fail
    log_info "Testing empty concept ID"
    if ! csm inject "" > /dev/null 2>&1; then
        log_success "Empty ID correctly rejected"
    else
        log_error "Empty ID should be rejected"
    fi
    
    # Test 2d: Special characters in ID (JSON escaping test)
    log_info "Testing special characters in concept ID"
    SPECIAL_ID='test"with\quotes'
    if csm inject "$SPECIAL_ID" > /dev/null 2>&1; then
        # Verify retrievable
        if csm probe "$SPECIAL_ID" -k 1 > /dev/null 2>&1; then
            log_success "Special character ID stored and retrievable"
        else
            log_error "Special character ID not retrievable"
        fi
    else
        log_error "Special character ID should be accepted"
    fi
}

# ============================================================================
# Test 3: Association validation
# ============================================================================
test_associations() {
    log_section "Test 3: Association Record Validation"
    
    # Create test concepts
    csm inject assoc_source > /dev/null 2>&1
    csm inject assoc_target > /dev/null 2>&1
    
    # Test 3a: Valid association
    log_info "Creating valid association"
    if csm associate assoc_source assoc_target -s 0.9 > /dev/null 2>&1; then
        log_success "Valid association created"
        
        # Verify in export
        EXPORT=$(csm export -o - 2>/dev/null)
        if echo "$EXPORT" | grep -q "assoc_source" && echo "$EXPORT" | grep -q "assoc_target"; then
            log_success "Association found in export"
        else
            log_error "Association missing from export"
        fi
    else
        log_error "Valid association should succeed"
    fi
    
    # Test 3b: Self-association (allowed with warning)
    log_info "Testing self-association"
    csm inject self_concept > /dev/null 2>&1
    OUTPUT=$(csm associate self_concept self_concept -s 0.5 2>&1)
    if echo "$OUTPUT" | grep -q "warning\|Warning"; then
        log_success "Self-association allowed with warning"
    elif csm associate self_concept self_concept -s 0.5 > /dev/null 2>&1; then
        log_success "Self-association created"
    else
        log_error "Self-association handling"
    fi
    
    # Test 3c: Negative strength (should fail)
    log_info "Testing negative association strength"
    if ! csm associate assoc_source assoc_target -s -0.5 > /dev/null 2>&1; then
        log_success "Negative strength correctly rejected"
    else
        log_error "Negative strength should be rejected"
    fi
}

# ============================================================================
# Test 4: Import/Export roundtrip
# ============================================================================
test_import_export() {
    log_section "Test 4: Import/Export Roundtrip Validation"
    
    # Create test data
    csm inject roundtrip_1 > /dev/null 2>&1
    csm inject roundtrip_2 > /dev/null 2>&1
    csm associate roundtrip_1 roundtrip_2 -s 0.8 > /dev/null 2>&1
    
    # Export to temp file
    EXPORT_FILE=$(mktemp -t csm_export_XXXXXX.json)
    trap "rm -f '$EXPORT_FILE' '$DB_FILE'" EXIT
    
    log_info "Exporting to temporary file"
    if csm export -o "$EXPORT_FILE" > /dev/null 2>&1; then
        log_success "Export to file successful"
    else
        log_error "Export to file failed"
        rm -f "$EXPORT_FILE"
        return 1
    fi
    
    # Verify file contents
    if [ -s "$EXPORT_FILE" ]; then
        log_success "Export file created and non-empty"
    else
        log_error "Export file is empty"
        rm -f "$EXPORT_FILE"
        return 1
    fi
    
    # Create new database and import
    NEW_DB=$(mktemp -t csm_import_XXXXXX.db)
    trap "rm -f '$EXPORT_FILE' '$DB_FILE' '$NEW_DB'" EXIT
    
    log_info "Importing into new database"
    if cargo run --quiet --bin csm -- --database "$NEW_DB" import "$EXPORT_FILE" > /dev/null 2>&1; then
        log_success "Import successful"
    else
        log_error "Import failed"
        rm -f "$EXPORT_FILE" "$NEW_DB"
        return 1
    fi
    
    # Verify imported data
    log_info "Verifying imported data"
    if cargo run --quiet --bin csm -- --database "$NEW_DB" probe roundtrip_1 -k 1 > /dev/null 2>&1; then
        log_success "Imported concept retrievable"
    else
        log_error "Imported concept not retrievable"
    fi
    
    # Cleanup temp files
    rm -f "$EXPORT_FILE" "$NEW_DB"
}

# ============================================================================
# Test 5: Batch operations
# ============================================================================
test_batch_operations() {
    log_section "Test 5: Batch Operation Validation"
    
    # Test 5a: Multiple concept injection
    log_info "Testing batch concept injection"
    for i in {1..10}; do
        csm inject "batch_concept_$i" > /dev/null 2>&1
    done
    
    # Verify all exist
    FOUND=0
    for i in {1..10}; do
        if csm probe "batch_concept_$i" -k 1 > /dev/null 2>&1; then
            ((FOUND++))
        fi
    done
    
    if [ "$FOUND" -eq 10 ]; then
        log_success "All 10 batch concepts stored and retrievable"
    else
        log_error "Only $FOUND/10 batch concepts retrievable"
    fi
    
    # Test 5b: Probe with different top_k values
    log_info "Testing probe with various top_k values"
    for k in 1 5 10; do
        RESULT=$(csm probe batch_concept_1 -k "$k" 2>/dev/null | grep -c "CONCEPT\|SIMILARITY" || true)
        log_info "top_k=$k: found $RESULT results"
    done
    log_success "Various top_k values tested"
}

# ============================================================================
# Test 6: Database integrity
# ============================================================================
test_database_integrity() {
    log_section "Test 6: Database Integrity Checks"
    
    # Check if database file exists and has content
    if [ -f "$DB_FILE" ] && [ -s "$DB_FILE" ]; then
        log_success "Database file exists and has content"
        
        # Check file size
        SIZE=$(stat -f%z "$DB_FILE" 2>/dev/null || stat -c%s "$DB_FILE" 2>/dev/null || echo "unknown")
        log_info "Database size: $SIZE bytes"
    else
        log_error "Database file missing or empty"
    fi
    
    # Test export format validity
    log_info "Validating export JSON format"
    EXPORT=$(csm export -o - 2>/dev/null)
    if echo "$EXPORT" | python3 -m json.tool > /dev/null 2>&1; then
        log_success "Export produces valid JSON"
    else
        log_error "Export JSON is invalid"
    fi
}

# ============================================================================
# Main execution
# ============================================================================
main() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  libsql Database Record Validation${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
    
    # Build the CLI first
    log_info "Building CLI binary..."
    cargo build --quiet --bin csm
    
    # Run all tests
    test_basic_crud
    test_edge_cases
    test_associations
    test_import_export
    test_batch_operations
    test_database_integrity
    
    # Summary
    log_section "Validation Summary"
    echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
    echo ""
    
    if [ "$TESTS_FAILED" -eq 0 ]; then
        echo -e "${GREEN}✓ All validations passed!${NC}"
        echo -e "${GREEN}✓ libsql persistence is working correctly${NC}"
        exit 0
    else
        echo -e "${RED}✗ Some validations failed${NC}"
        exit 1
    fi
}

main "$@"
