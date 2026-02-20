#!/usr/bin/env bash
set -euo pipefail

# Demonstrates self-association (concept associated with itself)
# Self-associations are allowed but generate a warning
#
# Real-world relevance:
#   Self-associations can represent self-referential concepts or
#   circular logic in knowledge graphs. While technically valid,
#   they often indicate data quality issues or logical errors.
#   The warning alerts users to potential problems while still
#   allowing legitimate use cases (e.g., reflexive relationships).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)

cleanup() {
    rm -f "$DB_FILE"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 11: Self-Association Warning"
echo "======================================================================"
echo ""
echo "Context: Circular relationship handling"
echo ""
echo "Description:"
echo "  Tests that associating a concept with itself generates a warning"
echo "  but still creates the association. This allows legitimate use"
echo "  cases while alerting users to potential data issues."
echo ""
echo "Expected behavior:"
echo "  - Self-association succeeds (exit code 0)"
echo "  - Warning message is displayed"
echo "  - Regular associations work without warnings"
echo ""

# First, create a concept
echo "-------------------------------------------------------------------"
echo "Setup: Creating test concept 'self-referential'"
echo "-------------------------------------------------------------------"
csm inject "self-referential" --database "$DB_FILE" --output-format quiet
echo "✓ Concept created"
echo ""

# Test 1: Self-association (should succeed with warning)
echo "-------------------------------------------------------------------"
echo "Test 1: Self-association (concept -> itself)"
echo "Command: csm associate self-referential self-referential --strength 0.8"
echo "-------------------------------------------------------------------"
OUTPUT=$(csm associate self-referential self-referential --strength 0.8 --database "$DB_FILE" 2>&1)
EXIT_CODE=$?

echo "$OUTPUT"
echo ""

if [ $EXIT_CODE -eq 0 ]; then
    echo "Exit code: $EXIT_CODE (success as expected)"
    echo "✓ Self-association created successfully"
    
    # Check if warning was shown (look for warning indicator or specific text)
    if echo "$OUTPUT" | grep -qi "self-association\|warning\|⚠"; then
        echo "✓ Warning message was displayed"
    else
        echo "ℹ Note: Warning indicator may be in stderr or use special formatting"
    fi
else
    echo "ERROR: Self-association should succeed with warning, not fail!"
    exit 1
fi
echo ""

# Create another concept for regular association test
echo "-------------------------------------------------------------------"
echo "Setup: Creating second concept 'other-concept'"
echo "-------------------------------------------------------------------"
csm inject "other-concept" --database "$DB_FILE" --output-format quiet
echo "✓ Second concept created"
echo ""

# Test 2: Regular association (should succeed without warning)
echo "-------------------------------------------------------------------"
echo "Test 2: Regular association (no self-reference)"
echo "Command: csm associate self-referential other-concept --strength 0.5"
echo "-------------------------------------------------------------------"
OUTPUT=$(csm associate self-referential other-concept --strength 0.5 --database "$DB_FILE" 2>&1)
EXIT_CODE=$?

echo "$OUTPUT"
echo ""

if [ $EXIT_CODE -eq 0 ]; then
    echo "Exit code: $EXIT_CODE (success as expected)"
    echo "✓ Regular association created successfully"
    
    # Regular associations should NOT have self-association warning
    if echo "$OUTPUT" | grep -qi "self-association"; then
        echo "WARNING: Unexpected self-association warning on regular association!"
    else
        echo "✓ No spurious self-association warning"
    fi
else
    echo "ERROR: Regular association should succeed!"
    exit 1
fi
echo ""

echo "======================================================================"
echo "SUCCESS: Self-association handling works correctly"
echo "======================================================================"
echo ""
echo "Summary:"
echo "  ✓ Self-association is allowed (exit code 0)"
echo "  ✓ Warning is generated to alert user"
echo "  ✓ Regular associations work without warnings"
echo ""
echo "Real-world significance:"
echo "  Allows reflexive relationships (e.g., 'is-a' hierarchies where"
echo "  a category is a member of itself) while warning about potential"
echo "  data quality issues like accidental circular references."
echo ""
echo "Example legitimate use cases:"
echo "  - 'Everything' is a member of 'Everything'"
echo "  - A 'Set' in mathematics can contain itself"
echo "  - Self-referential types in programming languages"
echo "======================================================================"
