#!/usr/bin/env bash
# =============================================================================
# sync-docs.sh - Comprehensive documentation version synchronization
# =============================================================================
# Usage: ./scripts/sync-docs.sh [--dry-run] [--check]
#
# This script ensures ALL documentation files are synchronized with the
# version defined in Cargo.toml. Run this after any version bump.
#
# Files updated:
#   - wasm/package.json (exact version)
#   - README.md (major.minor for semver compatibility)
#   - book/src/getting-started.md (major.minor)
#   - SECURITY.md (add new supported version)
#   - llms.txt & llms-full.txt (regenerated)
#   - wasm/README.md (if exists)
#
# Exit codes:
#   0 - All files in sync
#   1 - Files were updated (or would be in dry-run)
#   2 - Error occurred
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

DRY_RUN=false
CHECK_ONLY=false

for arg in "$@"; do
    case $arg in
        --dry-run) DRY_RUN=true ;;
        --check) CHECK_ONLY=true ;;
        --help|-h)
            echo "Usage: $0 [--dry-run] [--check]"
            echo ""
            echo "Options:"
            echo "  --dry-run    Show what would be changed without modifying files"
            echo "  --check      Exit 1 if files need updates (for CI)"
            echo "  --help       Show this help"
            exit 0
            ;;
    esac
done

cd "${PROJECT_ROOT}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Counters
UPDATED=0
SKIPPED=0
FAILED=0

# Get version from Cargo.toml
CARGO_VERSION=$(grep -m1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
if [[ -z "$CARGO_VERSION" ]]; then
    echo -e "${RED}✗ Could not extract version from Cargo.toml${NC}"
    exit 2
fi

# Extract major.minor for semver compatibility in docs
MAJOR_MINOR=$(echo "$CARGO_VERSION" | cut -d. -f1,2)

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Documentation Version Sync${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Cargo.toml version: ${CARGO_VERSION}"
echo "Major.minor:        ${MAJOR_MINOR}"
echo ""

# Helper function to update a file
# Args: file, expected_version, pattern, replacement, description
update_file() {
    local file="$1"
    local expected_version="$2"
    local pattern="$3"
    local replacement="$4"
    local description="$5"

    if [[ ! -f "$file" ]]; then
        echo -e "${YELLOW}⊘${NC} $description: file not found ($file)"
        ((SKIPPED++)) || true
        return 0
    fi

    # Extract current version from file
    local current_version
    current_version=$(grep -oE "$pattern" "$file" 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' || true)

    # Check if already in sync
    if [[ "$current_version" == "$expected_version" ]] || [[ "$current_version" == "${expected_version%.*}" ]]; then
        echo -e "${GREEN}✓${NC} $description: already in sync ($current_version)"
        ((SKIPPED++)) || true
        return 0
    fi

    # Version mismatch or pattern not found
    if $CHECK_ONLY; then
        if [[ -n "$current_version" ]]; then
            echo -e "${YELLOW}!${NC} $description: needs update ($current_version → $expected_version)"
        else
            echo -e "${YELLOW}!${NC} $description: pattern not found"
        fi
        ((UPDATED++)) || true
        return 0
    fi

    if $DRY_RUN; then
        echo -e "${BLUE}○${NC} $description: would update"
        ((UPDATED++)) || true
    else
        sed -i "$replacement" "$file"
        echo -e "${GREEN}✓${NC} $description: updated"
        ((UPDATED++)) || true
    fi
}

# Helper to check if version exists in SECURITY.md supported versions
check_security_md() {
    local file="SECURITY.md"
    if [[ ! -f "$file" ]]; then
        echo -e "${YELLOW}⊘${NC} SECURITY.md: file not found"
        return 0
    fi

    # Check if this major.minor version is already listed
    if grep -q "| ${MAJOR_MINOR}\.x" "$file"; then
        echo -e "${GREEN}✓${NC} SECURITY.md: ${MAJOR_MINOR}.x already listed as supported"
        ((SKIPPED++)) || true
        return 0
    fi

    if $CHECK_ONLY; then
        echo -e "${YELLOW}!${NC} SECURITY.md: needs ${MAJOR_MINOR}.x added to supported versions"
        ((UPDATED++)) || true
        return 0
    fi

    if $DRY_RUN; then
        echo -e "${BLUE}○${NC} SECURITY.md: would add ${MAJOR_MINOR}.x to supported versions"
        ((UPDATED++)) || true
        return 0
    fi

    # Add new version row after the table header (find the separator line and insert after)
    # The table looks like:
    # | Version | Supported          |
    # | ------- | ------------------ |
    # | 0.1.x   | :white_check_mark: |
    sed -i "/| ------- | ------------------ |/a | ${MAJOR_MINOR}.x   | :white_check_mark: |" "$file"
    echo -e "${GREEN}✓${NC} SECURITY.md: added ${MAJOR_MINOR}.x to supported versions"
    ((UPDATED++)) || true
}

# =============================================================================
# 1. wasm/package.json - exact version
# =============================================================================
echo ""
echo -e "${BLUE}→ Checking wasm/package.json${NC}"
update_file "wasm/package.json" \
    "$CARGO_VERSION" \
    '"version": "[0-9]+\.[0-9]+\.[0-9]+"' \
    "s/\"version\": \"[0-9]\+\.[0-9]\+\.[0-9]\+\"/\"version\": \"${CARGO_VERSION}\"/" \
    "wasm/package.json version"

# =============================================================================
# 2. README.md - major.minor for semver compatibility
# =============================================================================
echo ""
echo -e "${BLUE}→ Checking README.md${NC}"
update_file "README.md" \
    "$MAJOR_MINOR" \
    'chaotic_semantic_memory = { version = "[0-9]+\.[0-9]+"' \
    "s/chaotic_semantic_memory = { version = \"[0-9]\+\.[0-9]\+\"/chaotic_semantic_memory = { version = \"${MAJOR_MINOR}\"/g" \
    "README.md installation version"

# =============================================================================
# 3. book/src/getting-started.md - major.minor
# =============================================================================
echo ""
echo -e "${BLUE}→ Checking book/src/getting-started.md${NC}"
update_file "book/src/getting-started.md" \
    "$MAJOR_MINOR" \
    'chaotic_semantic_memory = { version = "[0-9]+\.[0-9]+"' \
    "s/chaotic_semantic_memory = { version = \"[0-9]\+\.[0-9]\+\"/chaotic_semantic_memory = { version = \"${MAJOR_MINOR}\"/g" \
    "book/src/getting-started.md version"

# =============================================================================
# 4. SECURITY.md - add new supported version
# =============================================================================
echo ""
echo -e "${BLUE}→ Checking SECURITY.md${NC}"
check_security_md

# =============================================================================
# 5. wasm/README.md - if exists
# =============================================================================
echo ""
echo -e "${BLUE}→ Checking wasm/README.md${NC}"
if [[ -f "wasm/README.md" ]]; then
    # Check if there's a versioned npm package reference
    if grep -q '@d-o-hub/chaotic_semantic_memory@[0-9]' wasm/README.md 2>/dev/null; then
        update_file "wasm/README.md" \
            "$CARGO_VERSION" \
            '@d-o-hub/chaotic_semantic_memory@[0-9]+\.[0-9]+\.[0-9]+' \
            "s/@d-o-hub\/chaotic_semantic_memory@[0-9]\+\.[0-9]\+\.[0-9]\+/@d-o-hub\/chaotic_semantic_memory@${CARGO_VERSION}/g" \
            "wasm/README.md version"
    else
        echo -e "${GREEN}✓${NC} wasm/README.md: no versioned package reference (uses latest)"
        ((SKIPPED++)) || true
    fi
fi

# =============================================================================
# 6. Regenerate llms.txt files (SKIPPED - runs in GitHub workflows)
# =============================================================================
echo ""
echo -e "${BLUE}→ Regenerating llms.txt files${NC}"
echo -e "${YELLOW}○${NC} llms.txt: skipping local generation (handled by CI)"

# =============================================================================
# Summary
# =============================================================================
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Files updated: ${UPDATED}"
echo "Files skipped: ${SKIPPED}"
echo ""

if $CHECK_ONLY; then
    if [[ "$UPDATED" -gt 0 ]]; then
        echo -e "${YELLOW}⚠ Documentation is out of sync. Run: ./scripts/sync-docs.sh${NC}"
        exit 1
    else
        echo -e "${GREEN}✓ All documentation is in sync${NC}"
        exit 0
    fi
fi

if $DRY_RUN; then
    echo -e "${YELLOW}Dry run complete. Run without --dry-run to apply changes.${NC}"
    exit 1
fi

if [[ "$UPDATED" -gt 0 ]]; then
    echo -e "${GREEN}✓ Documentation synchronized to version ${CARGO_VERSION}${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Review changes: git diff"
    echo "  2. Commit: git add -A && git commit -m 'docs: sync version to ${CARGO_VERSION}'"
    exit 1
else
    echo -e "${GREEN}✓ All documentation already in sync${NC}"
    exit 0
fi
