#!/usr/bin/env bash
set -euo pipefail

# Demonstrates top_k parameter validation in probe queries
# top_k=0 should fail, very large values should be limited/rejected
#
# Real-world relevance:
#   top_k controls how many similar concepts are returned in a query.
#   Invalid values (0, extremely large) could cause:
#   - Empty results (top_k=0)
#   - Memory exhaustion or DoS (very large top_k)
#   Proper validation ensures system stability and meaningful results.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)

cleanup() {
    rm -f "$DB_FILE"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 10: Top-K Query Limits"
echo "======================================================================"
echo ""
echo "Context: Query result limiting"
echo ""
echo "Description:"
echo "  Tests validation of top_k parameter in probe queries."
echo "  top_k must be at least 1 and within reasonable limits."
echo ""
echo "Expected behavior:"
echo "  - top_k=0 is rejected (need at least 1 result)"
echo "  - Very large top_k exceeds limit and is rejected"
echo "  - Valid range (1-10000) works correctly"
echo ""

# First, create a concept to query
echo "-------------------------------------------------------------------"
echo "Setup: Creating test concept 'query-concept'"
echo "-------------------------------------------------------------------"
csm inject "query-concept" --database "$DB_FILE" --output-format quiet
echo "✓ Concept created"
echo ""

# Test 1: top_k=0 (should fail)
echo "-------------------------------------------------------------------"
echo "Test 1: top_k=0 (should fail - need at least 1 result)"
echo "Command: csm probe query-concept --top-k 0"
echo "-------------------------------------------------------------------"
if csm probe query-concept --top-k 0 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "ERROR: Command succeeded but should have failed!"
    exit 1
else
    EXIT_CODE=$?
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ top_k=0 correctly rejected"
fi
echo ""

# Test 2: Very large top_k (should fail or be limited)
echo "-------------------------------------------------------------------"
echo "Test 2: Very large top_k=99999 (exceeds limit)"
echo "Command: csm probe query-concept --top-k 99999"
echo "-------------------------------------------------------------------"
if csm probe query-concept --top-k 99999 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "ERROR: Command succeeded but should have failed (exceeds 10000 limit)!"
    exit 1
else
    EXIT_CODE=$?
    echo ""
    echo "Exit code: $EXIT_CODE (non-zero as expected)"
    echo "✓ Excessive top_k correctly rejected"
fi
echo ""

# Test 3: top_k=1 (minimum valid, should work)
echo "-------------------------------------------------------------------"
echo "Test 3: top_k=1 (minimum valid value)"
echo "Command: csm probe query-concept --top-k 1"
echo "-------------------------------------------------------------------"
if csm probe query-concept --top-k 1 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ top_k=1 correctly accepted"
else
    echo ""
    echo "ERROR: top_k=1 should be accepted!"
    exit 1
fi
echo ""

# Test 4: top_k=10 (default range, should work)
echo "-------------------------------------------------------------------"
echo "Test 4: top_k=10 (common default)"
echo "Command: csm probe query-concept --top-k 10"
echo "-------------------------------------------------------------------"
if csm probe query-concept --top-k 10 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ top_k=10 correctly accepted"
else
    echo ""
    echo "ERROR: top_k=10 should be accepted!"
    exit 1
fi
echo ""

# Test 5: top_k=10000 (maximum valid, should work)
echo "-------------------------------------------------------------------"
echo "Test 5: top_k=10000 (maximum allowed value)"
echo "Command: csm probe query-concept --top-k 10000"
echo "-------------------------------------------------------------------"
if csm probe query-concept --top-k 10000 --database "$DB_FILE" 2>&1; then
    echo ""
    echo "✓ top_k=10000 correctly accepted"
else
    echo ""
    echo "ERROR: top_k=10000 should be accepted!"
    exit 1
fi
echo ""

echo "======================================================================"
echo "SUCCESS: Top-k validation works correctly"
echo "======================================================================"
echo ""
echo "Summary:"
echo "  ✓ top_k=0 is rejected (minimum is 1)"
echo "  ✓ top_k=99999 is rejected (exceeds 10000 limit)"
echo "  ✓ top_k=1 is accepted (minimum valid)"
echo "  ✓ top_k=10 is accepted (common default)"
echo "  ✓ top_k=10000 is accepted (maximum valid)"
echo ""
echo "Real-world significance:"
echo "  Prevents resource exhaustion from queries requesting millions"
echo "  of results, while allowing flexible result set sizes for"
echo "  different use cases (top 5, top 100, etc.)."
echo "======================================================================"
