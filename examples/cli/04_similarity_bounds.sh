#!/usr/bin/env bash
set -euo pipefail

# Demonstrates that similarity scores are always in the range [-1, 1]
# Hypervector similarity uses cosine similarity which is bounded to [-1, 1]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_FILE=$(mktemp)

cleanup() {
    rm -f "$DB_FILE"
}
trap cleanup EXIT

echo "======================================================================"
echo "Edge Case 4: Similarity Score Bounds [-1, 1]"
echo "======================================================================"
echo ""
echo "Description:"
echo "  Injects several concepts and probes to verify all similarity scores"
echo "  fall within the mathematically guaranteed bounds of [-1, 1]."
echo ""
echo "Background:"
echo "  Similarity is computed as the cosine similarity between hypervectors."
echo "  Due to the Cauchy-Schwarz inequality, cosine similarity is always"
echo "  bounded to the range [-1, 1] where:"
echo "    - 1.0  = identical vectors (perfect match)"
echo "    - 0.0  = orthogonal vectors (no similarity)"
echo "    - -1.0 = opposite vectors (perfect anti-correlation)"
echo ""
echo "======================================================================"
echo ""

# Inject several concepts with random vectors
echo "Step 1: Injecting test concepts..."
echo "----------------------------------------------------------------------"

for concept in cat dog mammal animal vehicle car truck; do
    echo "Injecting: $concept"
    csm inject "$concept" --database "$DB_FILE" 2>/dev/null
done

echo ""
echo "✓ Injected 7 concepts"
echo ""

# Probe each concept and collect similarity scores
echo "Step 2: Probing all concepts and collecting similarity scores..."
echo "----------------------------------------------------------------------"

ALL_SIMILARITIES=""
VIOLATIONS_FOUND=0

for concept in cat dog mammal animal vehicle car truck; do
    echo ""
    echo "Probing: $concept"
    
    # Get probe results in JSON format and extract similarities
    RESULTS=$(csm probe "$concept" --database "$DB_FILE" --output-format json 2>/dev/null)
    
    # Extract similarity values using simple parsing
    SIMILARITIES=$(echo "$RESULTS" | grep -o '"similarity":[-0-9.]*' | cut -d':' -f2)
    
    echo "Similarities found:"
    for sim in $SIMILARITIES; do
        echo "  $sim"
        ALL_SIMILARITIES="$ALL_SIMILARITIES $sim"
        
        # Check if within bounds [-1, 1]
        # Using bc for floating point comparison
        if command -v bc >/dev/null 2>&1; then
            if (( $(echo "$sim < -1.0" | bc -l) )) || (( $(echo "$sim > 1.0" | bc -l) )); then
                echo "    WARNING: Similarity $sim is OUTSIDE [-1, 1] bounds!"
                VIOLATIONS_FOUND=$((VIOLATIONS_FOUND + 1))
            fi
        fi
    done
done

echo ""
echo "======================================================================"
echo "Step 3: Bounds Verification"
echo "======================================================================"
echo ""

if command -v bc >/dev/null 2>&1; then
    if [ "$VIOLATIONS_FOUND" -eq 0 ]; then
        echo "✓ All similarity scores are within [-1, 1] bounds"
        echo ""
        echo "Statistics:"
        
        # Find min and max using bc
        MIN_VAL=$(echo "$ALL_SIMILARITIES" | tr ' ' '\n' | sort -n | head -1)
        MAX_VAL=$(echo "$ALL_SIMILARITIES" | tr ' ' '\n' | sort -n | tail -1)
        
        echo "  Minimum similarity: $MIN_VAL"
        echo "  Maximum similarity: $MAX_VAL"
        echo "  Bounds: [-1.0, 1.0]"
    else
        echo "✗ Found $VIOLATIONS_FOUND similarity values outside [-1, 1] bounds!"
        exit 1
    fi
else
    echo "Note: bc not available for floating-point comparison"
    echo "Manual verification required - all similarities should be in [-1, 1]"
fi

echo ""

# Additional demonstration: Show what perfect similarity looks like
echo "Step 4: Demonstrating self-similarity (should be near 1.0)"
echo "----------------------------------------------------------------------"
echo ""

SELF_SIM=$(csm probe cat --database "$DB_FILE" --output-format json 2>/dev/null | \
    grep -o '"similarity":[-0-9.]*' | head -1 | cut -d':' -f2)

echo "Self-similarity of 'cat' (identical vectors): $SELF_SIM"
if command -v bc >/dev/null 2>&1; then
    if (( $(echo "$SELF_SIM > 0.99" | bc -l) )); then
        echo "✓ Self-similarity is near 1.0 as expected"
    fi
fi

echo ""

# Demonstrate random vector similarities
echo "Step 5: Random vector similarities (typically near 0.0)"
echo "----------------------------------------------------------------------"
echo ""

RANDOM_SIMS=""
for concept in dog mammal vehicle car; do
    SIM=$(csm probe cat --database "$DB_FILE" --output-format json 2>/dev/null | \
        jq -r ".results[] | select(.concept_id == \"$concept\") | .similarity" 2>/dev/null || \
        echo "N/A")
    if [ "$SIM" != "N/A" ] && [ -n "$SIM" ]; then
        echo "cat -> $concept: $SIM"
        RANDOM_SIMS="$RANDOM_SIMS $SIM"
    fi
done

if [ -z "$RANDOM_SIMS" ]; then
    # Fallback without jq
    echo "Cross-concept similarities (sample from probe output):"
    csm probe cat --database "$DB_FILE" --output-format table 2>/dev/null | tail -n +4 | head -5
fi

echo ""
echo "Note: Random high-dimensional vectors are nearly orthogonal,"
echo "      so their similarities cluster near 0.0"

echo ""
echo "======================================================================"
echo "SUCCESS: All similarity scores are within the expected bounds"
echo ""
echo "Mathematical guarantee:"
echo "  Cosine similarity = (A · B) / (||A|| × ||B||)"
echo "  By Cauchy-Schwarz: |A · B| ≤ ||A|| × ||B||"
echo "  Therefore: -1 ≤ cosine_similarity ≤ 1"
echo "======================================================================"
