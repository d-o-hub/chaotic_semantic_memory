#!/usr/bin/env bash
#
# run_all_edge_case_examples.sh
#
# Runs all CLI edge case examples and validates they work correctly.
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure csm is available
if ! command -v csm > /dev/null 2>&1; then
    echo "Installing csm binary to ~/.local/bin..."
    mkdir -p ~/.local/bin
    cargo build --release --bin csm
    cp target/release/csm ~/.local/bin/
    export PATH="$HOME/.local/bin:$PATH"
fi

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0

# List of all edge case scripts
SCRIPTS=(
    "01_empty_concept_id.sh"
    "02_oversized_concept_id.sh"
    "03_invalid_vector_format.sh"
    "04_similarity_bounds.sh"
    "05_empty_sequence.sh"
    "06_spectral_radius_bounds.sh"
    "07_reservoir_size_limits.sh"
    "08_sequence_temporal.sh"
    "09_negative_strength.sh"
    "10_top_k_limits.sh"
    "11_self_association.sh"
    "12_metadata_limits.sh"
    "13_import_missing_file.sh"
    "14_duplicate_concept.sh"
    "15_export_empty_db.sh"
    "16_batch_operations.sh"
)

echo "========================================"
echo "  Running CLI Edge Case Examples"
echo "========================================"
echo ""

for script in "${SCRIPTS[@]}"; do
    script_path="$SCRIPT_DIR/$script"
    if [ -f "$script_path" ]; then
        echo -e "${YELLOW}Running: $script${NC}"
        if timeout 30 bash "$script_path" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ PASS${NC}: $script"
            ((PASSED++))
        else
            echo -e "${RED}✗ FAIL${NC}: $script"
            ((FAILED++))
        fi
    else
        echo -e "${RED}✗ MISSING${NC}: $script"
        ((FAILED++))
    fi
done

echo ""
echo "========================================"
echo "           Summary"
echo "========================================"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All edge case examples passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some examples failed${NC}"
    exit 1
fi
