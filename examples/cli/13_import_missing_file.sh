#!/usr/bin/env bash
#
# 13_import_missing_file.sh - Demonstrate import of non-existent file
#
# Context: Common file handling mistake
# Shows what happens when you try to import from a file that doesn't exist,
# then demonstrates the correct approach with a valid file.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create temporary directory for this demo
TEMP_DIR=$(mktemp -d)
DB_FILE="$TEMP_DIR/demo.db"
VALID_IMPORT_FILE="$TEMP_DIR/valid_export.json"
NONEXISTENT_FILE="$TEMP_DIR/this_file_does_not_exist.json"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}  Demo 13: Import Missing File${NC}"
echo -e "${BLUE}  Context: Common file handling mistake${NC}"
echo -e "${BLUE}===============================================${NC}"
echo ""

# Build the CLI if needed
echo -e "${BLUE}Building CLI...${NC}"
cargo build --bin csm --features cli --quiet 2>/dev/null || true
echo ""

# First, create a valid import file manually (export format has known serialization issue)
echo -e "${YELLOW}Step 1: Create a valid import file for comparison${NC}"
echo "  Creating valid import JSON..."

# Create a valid export format JSON with empty concepts/associations
# This demonstrates the import structure without serialization issues
cat > "$VALID_IMPORT_FILE" << 'EOF'
{
  "version": "0.2.6",
  "exported_at": 1700000000,
  "concepts": [],
  "associations": []
}
EOF

echo "  Valid import file created: $VALID_IMPORT_FILE"
echo ""

# Demonstrate the error case
echo -e "${YELLOW}Step 2: Attempt to import from non-existent file${NC}"
echo -e "  Command: csm import ${NONEXISTENT_FILE}"
echo "  Expected: Error with clear message about missing file"
echo ""

set +e
./target/debug/csm import "$NONEXISTENT_FILE" 2>&1
EXIT_CODE=$?
set -e

echo ""
echo -e "  Exit code: ${RED}$EXIT_CODE${NC} (non-zero indicates error)"
echo ""

# Now show the success case
echo -e "${YELLOW}Step 3: Import from valid file (success case)${NC}"
echo -e "  Command: csm import ${VALID_IMPORT_FILE}"
echo ""

# Create fresh database for import
rm -f "$DB_FILE"
./target/debug/csm --database "$DB_FILE" import "$VALID_IMPORT_FILE"
echo ""

# Summary
echo -e "${GREEN}===============================================${NC}"
echo -e "${GREEN}  Summary${NC}"
echo -e "${GREEN}===============================================${NC}"
echo ""
echo "Key takeaways:"
echo "  1. Importing a non-existent file returns a clear error message"
echo "  2. Exit code is non-zero (failure) when file is missing"
echo "  3. Always verify file existence before import in scripts"
echo "  4. The error distinguishes between 'file not found' and other errors"
echo ""
echo "Real-world usage:"
echo "  - Always check if backup files exist before restore operations"
echo "  - Use 'test -f <file>' in scripts to validate before import"
echo "  - Handle the error gracefully in automation pipelines"
echo ""
echo "Import file format:"
echo '  {'
echo '    "version": "0.2.6",'
echo '    "exported_at": <unix_timestamp>,'
echo '    "concepts": [...],'
echo '    "associations": [...]'
echo '  }'
echo ""
