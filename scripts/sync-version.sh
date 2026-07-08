#!/bin/bash
# =============================================================================
# sync-version.sh - Automate version updates for releases
# =============================================================================
# Usage: ./scripts/sync-version.sh <version> [--dry-run]
# Example: ./scripts/sync-version.sh 0.2.0
# =============================================================================

set -e

VERSION="$1"
DRY_RUN=""
if [ "$2" = "--dry-run" ]; then
    DRY_RUN="true"
fi

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version> [--dry-run]"
    exit 1
fi

# Validate version format (semver) to prevent command injection
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Version must be in semver format (e.g., 0.2.0)"
  exit 1
fi

# Get current version from Cargo.toml first
CURRENT_VERSION=$(grep -m1 '^version = ' "Cargo.toml" | sed 's/^version = "\(.*\)"/\1/')

# If version unchanged, skip validation and exit early
if [ "$VERSION" = "$CURRENT_VERSION" ]; then
    echo "Version $VERSION is current. No sync needed."
    exit 0
fi

# Extract major.minor for Cargo.toml compatibility version
MAJOR_MINOR=$(echo "$VERSION" | cut -d. -f1,2)

echo "=============================================="
echo "Version Sync: $VERSION"
echo "=============================================="

# Files that need version updates
declare -A VERSION_FILES
VERSION_FILES["Cargo.toml"]="s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/"
VERSION_FILES["README.md"]="s/version = \"$CURRENT_VERSION\"/version = \"$MAJOR_MINOR\"/g"
VERSION_FILES["book/src/getting-started.md"]="s/version = \"$CURRENT_VERSION\"/version = \"$MAJOR_MINOR\"/g"
VERSION_FILES["wasm/package.json"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/"
VERSION_FILES["cli-npm/package.json"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/"
VERSION_FILES["tests/framework_lifecycle.rs"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g"
VERSION_FILES["examples/cli/12_metadata_limits.sh"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g"
VERSION_FILES["examples/cli/13_import_missing_file.sh"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g"
VERSION_FILES["examples/cli/16_batch_operations.sh"]="s/\"version\":\"$CURRENT_VERSION\"/\"version\":\"$VERSION\"/g"

if [ -n "$DRY_RUN" ]; then
    echo "DRY RUN - No files modified"
    for file in "${!VERSION_FILES[@]}"; do
        if [ -f "$file" ]; then
            echo "  Would update $file: ${VERSION_FILES[$file]}"
        fi
    done
    echo "  Would update all crates/*/Cargo.toml: $CURRENT_VERSION → $VERSION"
    exit 0
fi

# Update Cargo.lock
cargo lock --version "$VERSION" 2>/dev/null || cargo update

# Update each file
for file in "${!VERSION_FILES[@]}"; do
    if [ -f "$file" ]; then
        # Ensure sed command is constructed safely
        sed -i "${VERSION_FILES[$file]}" "$file"
        echo "  ✓ Updated $file"
    fi
done

# Update workspace crate versions (Cargo.toml files under crates/)
echo "  Updating workspace crates..."
find crates -name "Cargo.toml" -exec sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/g" {} \;
echo "  ✓ Updated workspace crate versions"

# Generate llms.txt files
if [ -f "scripts/gen-llms-txt.sh" ]; then
    bash scripts/gen-llms-txt.sh || echo "Warning: llms.txt generation failed"
fi

echo "Version sync complete: $CURRENT_VERSION → $VERSION"
