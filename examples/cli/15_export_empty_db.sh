#!/usr/bin/env bash
#
# 15_export_empty_db.sh - Demonstrate empty database export
#
# Context: Backup of fresh system
# Shows that exporting an empty database produces a valid export file
# with appropriate warnings, useful for system initialization and testing.
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
DB_FILE="$TEMP_DIR/empty.db"
EXPORT_FILE="$TEMP_DIR/export.json"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo -e "${BLUE}===============================================${NC}"
echo -e "${BLUE}  Demo 15: Export Empty Database${NC}"
echo -e "${BLUE}  Context: Backup of fresh system${NC}"
echo -e "${BLUE}===============================================${NC}"
echo ""

# Build the CLI if needed
echo -e "${BLUE}Building CLI...${NC}"
cargo build --bin csm --features cli --quiet 2>/dev/null || true
echo ""

# Verify database is truly empty (fresh creation)
echo -e "${YELLOW}Step 1: Create fresh empty database${NC}"
echo "  Database path: $DB_FILE"
echo "  (Database will be created on first access)"
echo ""

# Export from empty database
echo -e "${YELLOW}Step 2: Export from empty database${NC}"
echo "  Command: csm --database <db> export --output export.json"
echo "  Expected: Warning about empty state, but successful export"
echo ""
./target/debug/csm --database "$DB_FILE" export --output "$EXPORT_FILE"
EXIT_CODE=$?
echo ""
echo -e "  Exit code: ${GREEN}$EXIT_CODE${NC} (0 = success)"
echo ""

# Show the resulting JSON structure
echo -e "${YELLOW}Step 3: Examine the exported JSON structure${NC}"
echo "  File: $EXPORT_FILE"
echo "  Contents:"
echo -e "${CYAN}"
cat "$EXPORT_FILE" | python3 -m json.tool 2>/dev/null || cat "$EXPORT_FILE"
echo -e "${NC}"
echo ""

# Verify the export is valid by importing it into another database
echo -e "${YELLOW}Step 4: Verify export validity by importing into new database${NC}"
echo "  Creating new database and importing the empty export..."
NEW_DB="$TEMP_DIR/new_empty.db"
./target/debug/csm --database "$NEW_DB" import "$EXPORT_FILE"
echo ""

# Compare with non-empty export
echo -e "${YELLOW}Step 5: Compare with non-empty export${NC}"
echo "  Adding some data and exporting again..."
./target/debug/csm --database "$DB_FILE" inject "concept1" 2>/dev/null
./target/debug/csm --database "$DB_FILE" inject "concept2" 2>/dev/null
./target/debug/csm --database "$DB_FILE" associate "concept1" "concept2" --strength 0.8 2>/dev/null
./target/debug/csm --database "$DB_FILE" export --output "$TEMP_DIR/non_empty.json" 2>/dev/null
echo ""
echo "  Empty export structure vs Non-empty export structure:"
echo -e "${CYAN}"
echo "  Empty:    $(cat "$EXPORT_FILE" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(f\"concepts={len(d[chr(99)+chr(111)+chr(110)+chr(99)+chr(101)+chr(112)+chr(116)+chr(115)])}, associations={len(d[chr(97)+chr(115)+chr(115)+chr(111)+chr(99)+chr(105)+chr(97)+chr(116)+chr(105)+chr(111)+chr(110)+chr(115)])}\")' 2>/dev/null || echo 'N/A')"
echo "  Non-empty: $(cat "$TEMP_DIR/non_empty.json" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(f"concepts={len(d[chr(99)+chr(111)+chr(110)+chr(99)+chr(101)+chr(112)+chr(116)+chr(115)])}, associations={len(d[chr(97)+chr(115)+chr(115)+chr(111)+chr(99)+chr(105)+chr(97)+chr(116)+chr(105)+chr(111)+chr(110)+chr(115)])}")' 2>/dev/null || echo 'N/A')"
echo -e "${NC}"
echo ""

# JSON format output for empty export
echo -e "${YELLOW}Step 6: JSON format output${NC}"
echo "  Exporting empty database with JSON output format:"
./target/debug/csm --database "$NEW_DB" export --output "$TEMP_DIR/json_export.json" --output-format json
echo ""

# Summary
echo -e "${GREEN}===============================================${NC}"
echo -e "${GREEN}  Summary${NC}"
echo -e "${GREEN}===============================================${NC}"
echo ""
echo "Key takeaways:"
echo "  1. Empty database exports succeed (exit code 0)"
echo "  2. Warning is displayed about empty state"
echo "  3. Valid JSON structure is produced with empty arrays"
echo "  4. Empty exports can be re-imported successfully"
echo ""
echo "Export structure:"
echo '  {'
echo '    "version": "<schema_version>",'
echo '    "exported_at": <unix_timestamp>,'
echo '    "concepts": [],'
echo '    "associations": []'
echo '  }'
echo ""
echo "Real-world usage:"
echo "  - System initialization: Create template empty state files"
echo "  - Testing: Verify backup/restore handles empty databases"
echo "  - CI/CD: Ensure export works even before data ingestion"
echo "  - Documentation: Show expected export format"
echo ""
