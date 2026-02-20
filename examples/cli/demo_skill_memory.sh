#!/bin/bash
#
# demo_skill_memory.sh - Real demonstration of skill-memory system
#
# This script simulates a real development workflow using skill-memory
# to track architectural decisions, code refactorings, and error resolutions.
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Skill Memory Demo - Dogfooding CSM${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Setup: Clean database for fresh demo
DB=".agents/memory/demo-skill-memory.db"
rm -f "$DB"
export CSM_MEMORY_DB="$DB"

# Load skill memory library
echo -e "${YELLOW}[1/5] Loading skill-memory library...${NC}"
source .opencode/lib/skill-memory.sh
echo -e "${GREEN}✓${NC} Library loaded"
echo ""

# =============================================================================
# SCENARIO 1: ADR Creation with Precedent Lookup
# =============================================================================
echo -e "${YELLOW}[2/5] Scenario 1: ADR Creation${NC}"
echo "Task: Creating ADR-0044 about implementing caching layer"
echo ""

# First, let's see if there are any related past decisions
echo "→ Checking for related architectural decisions..."
PRECEDENTS=$(skill_recall "caching performance" 0.6 3)
PRECEDENT_COUNT=$(echo "$PRECEDENTS" | jq 'length')

if [ "$PRECEDENT_COUNT" -eq 0 ]; then
    echo -e "${BLUE}  ℹ${NC} No precedents found (fresh database)"
else
    echo -e "${GREEN}  ✓${NC} Found $PRECEDENT_COUNT related decisions:"
    echo "$PRECEDENTS" | jq -r '.[] | "    - \(.metadata.operation): \(.metadata.context)"'
fi
echo ""

# Create a new ADR
echo "→ Creating ADR-0044: Cache Layer Implementation"
ADR_ID=$(skill_remember "adr-creation" \
    "architectural_decision" \
    "ADR-0044: Implement LRU cache for concept storage" \
    "Decision: Use lru crate with 128 entry limit. Rationale: Prevents memory blow-up while maintaining performance.")

echo -e "${GREEN}✓${NC} Stored ADR in memory: ${ADR_ID:0:60}..."
echo ""

# =============================================================================
# SCENARIO 2: Code Refactoring with Pattern Memory
# =============================================================================
echo -e "${YELLOW}[3/5] Scenario 2: Code Refactoring${NC}"
echo "Task: Refactoring error handling to use thiserror"
echo ""

# Check for similar refactorings
echo "→ Looking for similar refactoring patterns..."
REFACTORINGS=$(skill_recall "error handling refactoring" 0.6 3)
REF_COUNT=$(echo "$REFACTORINGS" | jq 'length')

if [ "$REF_COUNT" -eq 0 ]; then
    echo -e "${BLUE}  ℹ${NC} No similar refactorings found"
else
    echo -e "${GREEN}✓${NC} Found $REF_COUNT similar refactorings"
fi
echo ""

# Remember the refactoring
echo "→ Recording refactoring operation..."
REFACTOR_ID=$(skill_remember "rust-development" \
    "code_refactoring" \
    "Converted custom error types to use thiserror derive macros in src/error.rs" \
    "Success: Reduced boilerplate by 40 lines, all tests pass, improved error messages")

echo -e "${GREEN}✓${NC} Stored refactoring: ${REFACTOR_ID:0:60}..."
echo ""

# Associate the refactoring with the ADR (related work)
echo "→ Linking refactoring to ADR (associated improvements)..."
skill_associate "$REFACTOR_ID" "$ADR_ID" 0.7
echo -e "${GREEN}✓${NC} Created association: refactoring → ADR (strength: 0.7)"
echo ""

# =============================================================================
# SCENARIO 3: Error Resolution with Solution Tracking
# =============================================================================
echo -e "${YELLOW}[4/5] Scenario 3: Error Resolution${NC}"
echo "Task: Debugging and fixing spectral radius issue"
echo ""

# Remember the error
echo "→ Recording error pattern..."
ERROR_ID=$(skill_remember "debugging-reservoir" \
    "error_resolution" \
    "Spectral radius > 1.1 causing unstable reservoir dynamics (chaotic divergence)" \
    "Root cause: User configured radius=1.2. Solution: Clamp to valid range [0.9, 1.1] with validation error.")

echo -e "${GREEN}✓${NC} Stored error: ${ERROR_ID:0:60}..."
echo ""

# Remember the fix separately
echo "→ Recording solution..."
SOLUTION_ID=$(skill_remember "debugging-reservoir" \
    "error_resolution" \
    "Added validation to set_spectral_radius() to enforce [0.9, 1.1] bounds with descriptive error message" \
    "Fix deployed: Tests added, validation works, prevents future misconfigurations")

echo -e "${GREEN}✓${NC} Stored solution: ${SOLUTION_ID:0:60}..."
echo ""

# Link error to solution
echo "→ Creating error → solution association..."
skill_associate "$ERROR_ID" "$SOLUTION_ID" 0.95
echo -e "${GREEN}✓${NC} Linked error → solution (strength: 0.95)"
echo ""

# Also link solution to ADR (if related to overall architecture)
skill_associate "$SOLUTION_ID" "$ADR_ID" 0.6
echo -e "${GREEN}✓${NC} Linked solution → ADR (strength: 0.6)"
echo ""

# =============================================================================
# SCENARIO 4: Test Failure Pattern
# =============================================================================
echo -e "${YELLOW}[5/5] Scenario 4: Test Failure Analysis${NC}"
echo "Task: Investigating intermittent test failure"
echo ""

echo "→ Recording test failure pattern..."
TEST_ID=$(skill_remember "testing-validation" \
    "test_failure" \
    "Intermittent failure in reservoir_step benchmark - timing variance on CI runners" \
    "Investigation: Added warmup iterations, increased tolerance, separate CI profile")

echo -e "${GREEN}✓${NC} Stored test failure: ${TEST_ID:0:60}..."
echo ""

# =============================================================================
# SUMMARY
# =============================================================================
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

skill_memory_stats
echo ""

echo -e "${GREEN}✓ Demo complete!${NC}"
echo ""
echo "Key achievements:"
echo "  • Stored 5 operations across 4 different skills"
echo "  • Created 3 associations linking related work"
echo "  • Demonstrated precedent lookup (recall)"
echo "  • All data persisted to libsql database"
echo ""
echo "Database location: $DB"
echo ""

# Export for verification
echo -e "${YELLOW}Exporting data for verification...${NC}"
csm --database "$DB" export -o /tmp/demo-memory-export.json
echo -e "${GREEN}✓${NC} Exported to: /tmp/demo-memory-export.json"
echo ""
