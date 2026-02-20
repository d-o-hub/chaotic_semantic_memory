#!/usr/bin/env bash
set -euo pipefail

# Demonstrates rejection of negative association strength values
# Negative strengths should fail validation, while zero and positive values work
#
# Real-world relevance:
#   Association strength represents relationship weight between concepts.
#   Negative values could represent "opposite" relationships, but in this
#   framework, strength is normalized to [0.0, ∞) to avoid ambiguity in
#   semantic similarity calculations. This prevents users from accidentally
#   creating nonsensical negative-weight associations.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)

cleanup() {
    rm -f "$DB_FILE"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 9: Negative Association Strength Rejection"
echo "======================================================================"
echo ""
echo "Context: Preventing invalid relationship weights"
echo ""
echo "Description:"
echo "  Tests that association strengths must be non-negative."
echo "  Negative values should be rejected with clear error messages."
echo ""
echo "Expected behavior:"
echo "  - Negative strength values (-0.5, -1.0) are rejected"
echo "  - Zero and positive values (0.0, 0.5, 1.0) are accepted"
echo ""

# First, create concepts to associate
echo "-------------------------------------------------------------------"
echo "Setup: Creating test concepts"
echo "-------------------------------------------------------------------"
csm inject "source-concept" --database "$DB_FILE" --output-format quiet
csm inject "target-concept" --database "$DB_FILE" --output-format quiet
csm inject "target-concept-2" --database "$DB_FILE" --output-format quiet
csm inject "target-concept-3" --database "$DB_FILE" --output-format quiet
echo "✓ Concepts created"
echo ""

# Test 1: Negative strength -0.5
echo "-------------------------------------------------------------------"
echo "Test 1: Negative strength (-0.5)"
echo "Command: csm associate source-concept target-concept --strength=-0.5"
echo "-------------------------------------------------------------------"
if csm associate source-concept target-concept --strength=-0.5 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    EXIT_CODE=$?
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Negative strength -0.5 correctly rejected"
fi
echo ""

# Test 2: Negative strength -1.0
echo "-------------------------------------------------------------------"
echo "Test 2: Negative strength (-1.0)"
echo "Command: csm associate source-concept target-concept --strength=-1.0"
echo "-------------------------------------------------------------------"
if csm associate source-concept target-concept --strength=-1.0 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    EXIT_CODE=$?
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Negative strength -1.0 correctly rejected"
fi
echo ""

# Test 3: Zero strength (should work)
echo "-------------------------------------------------------------------"
echo "Test 3: Zero strength (0.0) - should succeed"
echo "Command: csm associate source-concept target-concept --strength=0.0"
echo "-------------------------------------------------------------------"
if csm associate source-concept target-concept --strength=0.0 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ Zero strength correctly accepted"
else
    echo ""
    echo "ERROR: Zero strength should be accepted!"
    exit 1
fi
echo ""

# Test 4: Positive strength (should work)
echo "-------------------------------------------------------------------"
echo "Test 4: Positive strength (0.5) - should succeed"
echo "Command: csm associate source-concept target-concept-2 --strength=0.5"
echo "-------------------------------------------------------------------"
if csm associate "source-concept" "target-concept-2" --strength=0.5 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ Positive strength correctly accepted"
else
    echo ""
    echo "ERROR: Positive strength should be accepted!"
    exit 1
fi
echo ""

# Test 5: Strength of 1.0 (should work)
echo "-------------------------------------------------------------------"
echo "Test 5: Strength of 1.0 - should succeed"
echo "Command: csm associate source-concept target-concept-3 --strength=1.0"
echo "-------------------------------------------------------------------"
if csm associate "source-concept" "target-concept-3" --strength=1.0 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ Strength 1.0 correctly accepted"
else
    echo ""
    echo "ERROR: Strength 1.0 should be accepted!"
    exit 1
fi
echo ""

echo "======================================================================"
echo "SUCCESS: Association strength validation works correctly"
echo "======================================================================"
echo ""
echo "Summary:"
echo "  ✓ Negative strengths (-0.5, -1.0) are rejected"
echo "  ✓ Zero strength (0.0) is accepted"
echo "  ✓ Positive strengths (0.5, 1.0) are accepted"
echo ""
echo "Real-world significance:"
echo "  Ensures relationship weights follow semantic similarity conventions"
echo "  where 0 = no relationship and higher values = stronger relationship."
echo "======================================================================"
