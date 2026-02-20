#!/usr/bin/env bash
set -euo pipefail

# Demonstrates handling of malformed vector data
# Vector files must contain exactly 320 float values (1280 bytes total)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)
VECTOR_DIR=$(mktemp -d)

cleanup() {
    rm -f "$DB_FILE"
    rm -rf "$VECTOR_DIR"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 3: Invalid Vector Format Handling"
echo "======================================================================"
echo ""
echo "Description:"
echo "  Tests various malformed vector file formats."
echo "  Valid vectors require exactly 320 float values (1280 bytes)."
echo ""
echo "======================================================================"
echo ""

# Test 1: Wrong number of floats (too few)
echo "Test 3a: Too few floats (10 instead of 320)"
echo "----------------------------------------------------------------------"
echo "Creating vector file with 10 floats..."
echo "0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9 1.0" > "$VECTOR_DIR/too_few.txt"

echo "Command: csm inject test-too-few --from-file $VECTOR_DIR/too_few.txt --vector-source file --database $DB_FILE"
echo ""

set +e
OUTPUT=$(csm inject test-too-few --from-file "$VECTOR_DIR/too_few.txt" --vector-source file --database "$DB_FILE" 2>&1)
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -eq 0 ]; then
    echo "$OUTPUT"
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    echo "$OUTPUT"
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Too few floats correctly rejected"
fi

echo ""

# Test 2: Wrong number of floats (too many)
echo "Test 3b: Too many floats (400 instead of 320)"
echo "----------------------------------------------------------------------"
echo "Creating vector file with 400 floats..."
python3 -c "print(' '.join(['0.5'] * 400))" > "$VECTOR_DIR/too_many.txt" 2>/dev/null || \
    (for i in {1..400}; do echo -n "0.5 "; done; echo) > "$VECTOR_DIR/too_many.txt"

echo "Command: csm inject test-too-many --from-file $VECTOR_DIR/too_many.txt --vector-source file --database $DB_FILE"
echo ""

set +e
OUTPUT=$(csm inject test-too-many --from-file "$VECTOR_DIR/too_many.txt" --vector-source file --database "$DB_FILE" 2>&1)
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -eq 0 ]; then
    echo "$OUTPUT"
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    echo "$OUTPUT"
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Too many floats correctly rejected"
fi

echo ""

# Test 3: Non-numeric data
echo "Test 3c: Non-numeric data"
echo "----------------------------------------------------------------------"
echo "Creating vector file with non-numeric values..."
cat > "$VECTOR_DIR/non_numeric.txt" << 'EOF'
0.1 abc 0.3 def 0.5
EOF

echo "Command: csm inject test-non-numeric --from-file $VECTOR_DIR/non_numeric.txt --vector-source file --database $DB_FILE"
echo ""

set +e
OUTPUT=$(csm inject test-non-numeric --from-file "$VECTOR_DIR/non_numeric.txt" --vector-source file --database "$DB_FILE" 2>&1)
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -eq 0 ]; then
    echo "$OUTPUT"
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    echo "$OUTPUT"
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Non-numeric data correctly rejected"
fi

echo ""

# Test 4: Invalid JSON array (wrong size)
echo "Test 3d: Invalid JSON array (too few elements)"
echo "----------------------------------------------------------------------"
echo "Creating JSON vector file with 5 elements..."
echo '[0.1, 0.2, 0.3, 0.4, 0.5]' > "$VECTOR_DIR/bad_json.txt"

echo "Command: csm inject test-bad-json --from-file $VECTOR_DIR/bad_json.txt --vector-source file --database $DB_FILE"
echo ""

set +e
OUTPUT=$(csm inject test-bad-json --from-file "$VECTOR_DIR/bad_json.txt" --vector-source file --database "$DB_FILE" 2>&1)
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -eq 0 ]; then
    echo "$OUTPUT"
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    echo "$OUTPUT"
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Invalid JSON array correctly rejected"
fi

echo ""

# Test 5: Valid vector (should succeed) - prove the system works correctly
echo "Test 3e: Valid vector (320 floats - should succeed)"
echo "----------------------------------------------------------------------"
echo "Creating valid vector file with 320 floats..."
python3 -c "print(' '.join(['0.5'] * 320))" > "$VECTOR_DIR/valid.txt" 2>/dev/null || \
    (for i in {1..320}; do echo -n "0.5 "; done; echo) > "$VECTOR_DIR/valid.txt"

echo "Command: csm inject test-valid --from-file $VECTOR_DIR/valid.txt --vector-source file --database $DB_FILE"
echo ""

set +e
OUTPUT=$(csm inject test-valid --from-file "$VECTOR_DIR/valid.txt" --vector-source file --database "$DB_FILE" 2>&1)
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -eq 0 ]; then
    echo "$OUTPUT"
    echo ""
    echo "Exit code: $EXIT_CODE (success as expected)"
    echo "✓ Valid vector correctly accepted"
else
    echo "$OUTPUT"
    echo ""
    echo "ERROR: Valid vector should have been accepted but was rejected!"
    exit 1
fi

echo ""
echo "======================================================================"
echo "SUCCESS: Vector format validation works correctly"
echo "  - Wrong number of floats: REJECTED"
echo "  - Non-numeric data: REJECTED"
echo "  - Invalid JSON: REJECTED"
echo "  - Valid 320-float vector: ACCEPTED"
echo "======================================================================"
