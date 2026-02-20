#!/usr/bin/env bash
set -euo pipefail

# Demonstrates rejection of concept IDs exceeding 256 bytes
# Concept IDs are limited to 256 bytes maximum

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)

cleanup() {
    rm -f "$DB_FILE"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 2: Oversized Concept ID Rejection (257+ bytes)"
echo "======================================================================"
echo ""
echo "Description:"
echo "  Attempts to inject a concept with a 257-byte ID (exceeds 256-byte limit)."
echo "  The CLI should reject this with a validation error."
echo ""
echo "Expected behavior:"
echo "  - Command fails with non-zero exit code"
echo "  - Error message indicates 'concept ID too long (max 256 bytes, got 257)'"
echo ""

# Generate a 257-byte string
OVERSIZED_ID=$(python3 -c "print('a' * 257)" 2>/dev/null || printf 'a%.0s' {1..257})
ID_LENGTH=${#OVERSIZED_ID}

echo "Generated concept ID length: $ID_LENGTH bytes"
echo ""

# Verify we generated the right length
if [ "$ID_LENGTH" -ne 257 ]; then
    echo "ERROR: Failed to generate 257-byte string (got $ID_LENGTH bytes)"
    exit 1
fi

echo "-------------------------------------------------------------------"
echo "Command: csm inject <257-byte-id> --database $DB_FILE"
echo "-------------------------------------------------------------------"
echo ""

# Attempt to inject with oversized ID - this should fail
set +e
OUTPUT=$(csm inject "$OVERSIZED_ID" --database "$DB_FILE" 2>&1)
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
    echo ""
    echo "✓ Oversized concept ID was correctly rejected"
fi

# Also verify that exactly 256 bytes works
echo ""
echo "-------------------------------------------------------------------"
echo "Verifying 256-byte ID is accepted..."
echo "-------------------------------------------------------------------"
echo ""

VALID_ID=$(python3 -c "print('a' * 256)" 2>/dev/null || printf 'a%.0s' {1..256})
VALID_LENGTH=${#VALID_ID}
echo "Testing with $VALID_LENGTH-byte ID..."
echo ""

if csm inject "$VALID_ID" --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ 256-byte concept ID was accepted (at the limit)"
else
    echo ""
    echo "ERROR: 256-byte ID should have been accepted but was rejected!"
    exit 1
fi

echo ""
echo "======================================================================"
echo "SUCCESS: Concept ID length validation works correctly"
echo "  - 256 bytes: ACCEPTED (at limit)"
echo "  - 257 bytes: REJECTED (over limit)"
echo "======================================================================"
