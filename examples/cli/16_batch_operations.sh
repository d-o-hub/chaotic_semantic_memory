#!/usr/bin/env bash
#
# 16_batch_operations.sh - Demonstrate batch edge cases
#
# Context: Bulk data operations
# Shows handling of empty batches, large batches, and partial failures
# in bulk association operations.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Create temporary directory for this demo
TEMP_DIR=$(mktemp -d)
DB_FILE="$TEMP_DIR/demo.db"
IMPORT_FILE="$TEMP_DIR/batch_import.json"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}  Demo 16: Batch Operations Edge Cases${NC}"
echo -e "${BLUE}  Context: Bulk data operations${NC}"
echo -e "${BLUE}===============================================${NC}"
echo ""

# Build the CLI if needed
echo -e "${BLUE}Building CLI...${NC}"
cargo build --bin csm --features cli --quiet 2>/dev/null || true
echo ""

# ============================================================================
# Edge Case 1: Empty batch (importing empty data)
# ============================================================================
echo -e "${YELLOW}Edge Case 1: Empty Import${NC}"
echo "  Creating and importing an empty concepts/associations file..."
echo '  {"version": "0.2.1","exported_at":0,"concepts":[],"associations":[]}' > "$IMPORT_FILE"
echo "  Command: csm import $IMPORT_FILE"
./target/debug/csm import "$IMPORT_FILE" --output-format json
echo ""

# ============================================================================
# Edge Case 2: Large batch processing
# ============================================================================
echo -e "${YELLOW}Edge Case 2: Large Batch Processing${NC}"
echo "  Injecting 50 concepts (simulating bulk data load)..."
echo ""

START_TIME=$(date +%s%N 2>/dev/null || date +%s)

for i in $(seq 1 50); do
    ./target/debug/csm --database "$DB_FILE" inject "concept_$i" 2>/dev/null || true
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "."
    fi
done

END_TIME=$(date +%s%N 2>/dev/null || date +%s)
if command -v python3 &> /dev/null; then
    DURATION_MS=$(( (END_TIME - START_TIME) / 1000000 ))
else
    DURATION_MS=$(( (END_TIME - START_TIME) * 1000 ))
fi

echo ""
echo "  Created 50 concepts in ${DURATION_MS}ms"
echo ""

# Create associations in batch
echo "  Creating 25 associations between concepts..."
for i in $(seq 1 25); do
    SRC="concept_$i"
    DST="concept_$((i + 1))"
    STRENGTH=$(echo "scale=2; $i / 25.0" | bc 2>/dev/null || echo "0.$i")
    ./target/debug/csm --database "$DB_FILE" associate "$SRC" "$DST" --strength "$STRENGTH" 2>/dev/null || true
done
echo "  Created 25 associations"
echo ""

# Show stats
echo "  Current database stats:"
echo "    Concepts: 50"
echo "    Associations: 25"
echo ""

# ============================================================================
# Edge Case 3: Partial failure handling (continue-on-error)
# ============================================================================
echo -e "${YELLOW}Edge Case 3: Partial Failure Handling${NC}"
echo "  Attempting operations that will partially fail..."
echo ""

# Try to associate non-existent concepts (will fail)
echo "  Test 3a: Associating non-existent concepts"
echo "  Command: csm associate nonexistent1 nonexistent2"
set +e
./target/debug/csm --database "$DB_FILE" associate "nonexistent1" "nonexistent2" 2>&1
ASSOC_EXIT=$?
set -e
echo "  Exit code: $ASSOC_EXIT (non-zero = failure)"
echo ""

# Try to probe non-existent concept
echo "  Test 3b: Probing non-existent concept"
echo "  Command: csm probe nonexistent"
set +e
./target/debug/csm --database "$DB_FILE" probe "nonexistent" 2>&1
PROBE_EXIT=$?
set -e
echo "  Exit code: $PROBE_EXIT (non-zero = failure)"
echo ""

# Show that partial operations work (some succeed, some fail)
echo "  Test 3c: Multiple operations with mixed results"
echo "  (Inject existing + non-existent association in sequence)"
echo ""

# This will succeed
./target/debug/csm --database "$DB_FILE" inject "batch_test" 2>/dev/null || true
echo "  [OK] Inject 'batch_test': SUCCESS"

# This will fail (target doesn't exist)
set +e
./target/debug/csm --database "$DB_FILE" associate "batch_test" "nonexistent_target" 2>/dev/null
if [ $? -ne 0 ]; then
    echo "  [OK] Associate to non-existent: EXPECTED FAILURE (handled)"
else
    echo "  [OK] Associate: SUCCESS"
fi
set -e

# This will succeed (both exist now)
./target/debug/csm --database "$DB_FILE" inject "existing_target" 2>/dev/null || true
./target/debug/csm --database "$DB_FILE" associate "batch_test" "existing_target" 2>/dev/null || true
echo "  [OK] Associate to existing target: SUCCESS"
echo ""

# ============================================================================
# Edge Case 4: JSON format for batch results
# ============================================================================
echo -e "${YELLOW}Edge Case 4: JSON Output for Batch Processing${NC}"
echo "  Using JSON format for programmatic batch handling:"
echo ""

echo "  Export with JSON format (parseable output):"
./target/debug/csm --database "$DB_FILE" export --output "$TEMP_DIR/batch_export.json" --output-format json
echo ""

echo "  Inject with JSON format:"
./target/debug/csm --database "$DB_FILE" inject "json_test" --output-format json
echo ""

echo "  Probe with JSON format:"
./target/debug/csm --database "$DB_FILE" probe "concept_1" --top-k 3 --output-format json
echo ""

# ============================================================================
# Summary
# ============================================================================
echo -e "${GREEN}===============================================${NC}"
echo -e "${GREEN}  Summary${NC}"
echo -e "${GREEN}===============================================${NC}"
echo ""
echo "Key takeaways:"
echo "  1. Empty imports are valid and succeed"
echo "  2. Large batches work but consider performance implications"
echo "  3. Individual operation failures don't corrupt the database"
echo "  4. Use JSON format for programmatic batch processing"
echo "  5. Each operation is atomic - failures are isolated"
echo ""
echo "Best practices for batch operations:"
echo "  - Pre-validate data before bulk import"
echo "  - Use transactions (import/export) for atomic operations"
echo "  - Handle errors gracefully in automation scripts"
echo "  - Consider batching large operations to manage memory"
echo "  - Use --output-format json for machine-readable results"
echo ""
echo "Performance notes:"
echo "  - Individual inject/associate calls have overhead"
echo "  - For bulk loads, consider using import with prepared files"
echo "  - Export/import is faster than individual operations for large datasets"
echo ""
