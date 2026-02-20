#!/usr/bin/env bash
#
# 14_duplicate_concept.sh - Demonstrate update vs create behavior
#
# Context: Idempotent concept updates
# Shows that injecting a concept with the same ID updates the existing
# concept rather than failing, enabling idempotent operations.
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

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}  Demo 14: Duplicate Concept Handling${NC}"
echo -e "${BLUE}  Context: Idempotent concept updates${NC}"
echo -e "${BLUE}===============================================${NC}"
echo ""

# Build the CLI if needed
echo -e "${BLUE}Building CLI...${NC}"
cargo build --bin csm --features cli --quiet 2>/dev/null || true
echo ""

# Inject concept first time
echo -e "${YELLOW}Step 1: Inject 'test' concept with random vector A${NC}"
echo "  Command: csm --database <db> inject test"
./target/debug/csm --database "$DB_FILE" inject "test"
echo ""

# Probe to see initial similarity with itself
echo -e "${YELLOW}Step 2: Probe to establish baseline similarity${NC}"
echo "  Command: csm --database <db> probe test --top-k 5"
./target/debug/csm --database "$DB_FILE" probe "test" --top-k 5
echo ""

# Inject concept again (same ID)
echo -e "${YELLOW}Step 3: Inject 'test' concept again (same ID)${NC}"
echo "  Command: csm --database <db> inject test"
echo "  Note: This UPDATES the existing concept rather than failing"
echo ""
./target/debug/csm --database "$DB_FILE" inject "test"
echo ""

# Show JSON output to demonstrate the 'updated' status
echo -e "${YELLOW}Step 4: Show JSON output demonstrating 'updated' status${NC}"
echo "  Injecting again with JSON output:"
./target/debug/csm --database "$DB_FILE" inject "test" --output-format json
echo ""

# Create another concept to demonstrate the 'created' status
echo -e "${YELLOW}Step 5: Show 'created' status for new concept${NC}"
echo "  Injecting a new concept with JSON output:"
./target/debug/csm --database "$DB_FILE" inject "new_concept" --output-format json
echo ""

# Summary
echo -e "${GREEN}===============================================${NC}"
echo -e "${GREEN}  Summary${NC}"
echo -e "${GREEN}===============================================${NC}"
echo ""
echo "Key takeaways:"
echo "  1. Re-injecting a concept with the same ID UPDATES it (idempotent)"
echo "  2. No error occurs - the operation succeeds with 'updated' status"
echo "  3. The new vector immediately becomes active for all operations"
echo "  4. This enables safe retry logic and state convergence"
echo ""
echo "Real-world usage:"
echo "  - Configuration updates: Re-inject config with new values"
echo "  - Model versioning: Update concept embeddings after retraining"
echo "  - Sync operations: Idempotent sync from external data sources"
echo "  - Safe retries: Network failures can be retried without side effects"
echo ""
echo "JSON status values:"
echo "  {\"status\": \"created\", \"concept_id\": \"...\"}  - New concept"
echo "  {\"status\": \"updated\", \"concept_id\": \"...\"}  - Existing concept"
echo ""
