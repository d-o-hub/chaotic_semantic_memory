#!/usr/bin/env bash
set -euo pipefail

# Demonstrates rejection of empty concept IDs
# Empty concept IDs should fail with a clear error message

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)

cleanup() {
    rm -f "$DB_FILE"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 1: Empty Concept ID Rejection"
echo "======================================================================"
echo ""
echo "Description:"
echo "  Attempts to inject a concept with an empty string as the concept ID."
echo "  The CLI should reject this with a validation error."
echo ""
echo "Expected behavior:"
echo "  - Command fails with non-zero exit code"
echo "  - Error message indicates 'concept ID cannot be empty'"
echo ""
echo "-------------------------------------------------------------------"
echo "Command: csm inject \"\" --database $DB_FILE"
echo "-------------------------------------------------------------------"
echo ""

# Attempt to inject with empty ID - this should fail
set +e
OUTPUT=$(csm inject "" --database "$DB_FILE" 2>&1)
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
    echo "✓ Empty concept ID was correctly rejected"
fi

echo ""
echo "======================================================================"
echo "SUCCESS: Empty concept ID validation works correctly"
echo "======================================================================"
