#!/bin/bash
# =============================================================================
# sync-version.sh - Automate version updates for releases
# =============================================================================
# Usage: ./scripts/sync-version.sh <version> [--dry-run]
# Example: ./scripts/sync-version.sh 0.3.9
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

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Version must be in semver format (e.g., 0.3.9)"
  exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep -m1 "^version = " "Cargo.toml" | sed 's/^version = "\(.*\)"/\1/')

# Extract major.minor for Cargo.toml compatibility version
MAJOR_MINOR=$(echo "$VERSION" | cut -d. -f1,2)

echo "=============================================="
echo "Version Sync Target: $VERSION (Current Cargo.toml: $CURRENT_VERSION)"
echo "=============================================="

if [ -n "$DRY_RUN" ]; then
    echo "DRY RUN - No files modified"
    echo "  Target version: $VERSION"
    echo "  Would update Cargo.toml, VERSION, wasm/package.json, cli-npm/package.json"
    echo "  Would update workspace crate versions and internal workspace dependency requirements in root Cargo.toml"
    exit 0
fi

# 1. Update VERSION file
echo "$VERSION" > VERSION
echo "  ✓ Updated VERSION"

# 2. Update Cargo.toml root package version
if [ -n "$CURRENT_VERSION" ]; then
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/" Cargo.toml
fi
echo "  ✓ Updated Cargo.toml root version"

# 3. Update root Cargo.toml internal dependency versions
sed -i -E 's/(path = "crates\/csm-[^"]+", version = ")([^"]+)(")/\1'"$VERSION"'\3/g' Cargo.toml
echo "  ✓ Updated root Cargo.toml internal workspace dependency requirements"

# 4. Update documentation / examples version strings
sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$MAJOR_MINOR\"/g" README.md 2>/dev/null || true
if [ -f "book/src/getting-started.md" ]; then
    sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$MAJOR_MINOR\"/g" book/src/getting-started.md || true
fi

if [ -f "wasm/package.json" ]; then
    sed -i -E 's/"version": "[0-9]+\.[0-9]+\.[0-9]+"/"version": "'"$VERSION"'"/' wasm/package.json
    echo "  ✓ Updated wasm/package.json"
fi

if [ -f "cli-npm/package.json" ]; then
    sed -i -E 's/"version": "[0-9]+\.[0-9]+\.[0-9]+"/"version": "'"$VERSION"'"/' cli-npm/package.json
    echo "  ✓ Updated cli-npm/package.json"
fi

if [ -f "tests/framework_lifecycle.rs" ]; then
    sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g" tests/framework_lifecycle.rs || true
fi

for ex in examples/cli/12_metadata_limits.sh examples/cli/13_import_missing_file.sh examples/cli/16_batch_operations.sh; do
    if [ -f "$ex" ]; then
        sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/g" "$ex" || true
        sed -i "s/\"version\":\"$CURRENT_VERSION\"/\"version\":\"$VERSION\"/g" "$ex" || true
    fi
done

# 5. Update workspace crate versions
echo "  Updating workspace crates..."
find crates -name "Cargo.toml" -exec sed -i -E 's/^version = "[0-9]+\.[0-9]+\.[0-9]+"/version = "'"$VERSION"'"/g' {} +
echo "  ✓ Updated workspace crate versions"

# 6. Synchronize Cargo.lock AFTER manifest updates
echo "  Updating Cargo.lock..."
cargo check --workspace >/dev/null 2>&1 || cargo update
echo "  ✓ Synchronized Cargo.lock"

# 7. Generate llms.txt files
if [ -f "scripts/gen-llms-txt.sh" ]; then
    bash scripts/gen-llms-txt.sh || echo "Warning: llms.txt generation failed"
fi

echo "Version sync complete: target version is $VERSION"
