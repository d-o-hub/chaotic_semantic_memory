#!/bin/bash
# =============================================================================
# sync-version.sh - Automate version updates for releases
# =============================================================================
# Usage: ./scripts/sync-version.sh <version> [--dry-run]
# Example: ./scripts/sync-version.sh 0.2.0
# =============================================================================
#
# Version compatibility guide:
# - "0.1"   = compatible with any 0.1.x (RECOMMENDED)
# - "0.1.0" = exact version
# - "^0.1.0" = compatible with 0.1.x (explicit caret)
#
# For crates.io, using "0.1" ensures users get the latest patch within 0.1.x
# =============================================================================

set -e

VERSION="$1"
DRY_RUN=""
if [ "$2" = "--dry-run" ]; then
    DRY_RUN="true"
fi

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version> [--dry-run]"
    echo ""
    echo "Examples:"
    echo "  $0 0.2.0          # Update to version 0.2.0"
    echo "  $0 0.2.0 --dry-run # Preview changes without writing"
    exit 1
fi

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semver format (e.g., 0.2.0)"
    exit 1
fi

# Extract major.minor for Cargo.toml compatibility version
MAJOR_MINOR=$(echo "$VERSION" | cut -d. -f1,2)

echo "=============================================="
echo "Version Sync: $VERSION"
echo "=============================================="

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep -m1 'version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $CURRENT_VERSION"
echo ""

if [ "$VERSION" = "$CURRENT_VERSION" ]; then
    echo "Version unchanged. Nothing to do."
    exit 0
fi

# Files that need version updates
declare -A VERSION_FILES

# Cargo.toml - exact version for publishing
VERSION_FILES["Cargo.toml"]="s/version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/"

# README.md and docs - use major.minor for semver compatibility in examples
VERSION_FILES["README.md"]="s/version = \"$CURRENT_VERSION\"/version = \"$MAJOR_MINOR\"/g"
VERSION_FILES["book/src/getting-started.md"]="s/version = \"$CURRENT_VERSION\"/version = \"$MAJOR_MINOR\"/g"

# CHANGELOG, package.json, tests - exact version
VERSION_FILES["CHANGELOG.md"]="s/\[Unreleased\]/\[$VERSION\]/; s/## \[Unreleased\]/## [$VERSION] - $(date +%Y-%m-%d)/"
VERSION_FILES["wasm/package.json"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/"

# Test and example files - exact version
VERSION_FILES["tests/framework_lifecycle.rs"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g"
VERSION_FILES["examples/cli/12_metadata_limits.sh"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g"
VERSION_FILES["examples/cli/13_import_missing_file.sh"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g"
VERSION_FILES["examples/cli/16_batch_operations.sh"]="s/\"version\":\"$CURRENT_VERSION\"/\"version\":\"$VERSION\"/g"

echo "Files to update:"
for file in "${!VERSION_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✓ $file"
    else
        echo "  ✗ $file (not found)"
    fi
done
echo ""

if [ -n "$DRY_RUN" ]; then
    echo "DRY RUN - No files modified"
    echo ""
    echo "Would apply these changes:"
    for file in "${!VERSION_FILES[@]}"; do
        if [ -f "$file" ]; then
            echo "  $file: ${VERSION_FILES[$file]}"
        fi
    done
    exit 0
fi

# Update Cargo.lock
echo "Updating Cargo.lock..."
cargo lock --version "$VERSION" 2>/dev/null || cargo update

# Update each file
echo ""
echo "Applying updates..."
for file in "${!VERSION_FILES[@]}"; do
    if [ -f "$file" ]; then
        sed -i "${VERSION_FILES[$file]}" "$file"
        echo "  ✓ Updated $file"
    fi
done

# Regenerate llms.txt
echo ""
echo "Regenerating llms.txt..."
bash scripts/gen-llms-txt.sh 2>/dev/null || true

echo ""
echo "=============================================="
echo "Version sync complete: $CURRENT_VERSION → $VERSION"
echo "=============================================="
echo ""
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Run validation: ./scripts/validate.sh"
echo "  3. Commit: git add -A && git commit -m \"release: v$VERSION\""
echo "  4. Tag: git tag -a v$VERSION -m \"Release v$VERSION\""
echo "  5. Push: git push origin main && git push origin v$VERSION"
echo ""
