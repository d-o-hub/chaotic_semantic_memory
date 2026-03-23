#!/bin/bash
# verify-version-sync.sh - Ensure all version numbers are synchronized
# Run this before releases to catch version drift

set -e

# Extract version from Cargo.toml
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# Extract version from wasm/package.json
NPM_LOCAL_VERSION=$(grep '"version":' wasm/package.json | sed 's/.*"version": "\(.*\)".*/\1/')

echo "Cargo.toml version: $CARGO_VERSION"
echo "wasm/package.json version: $NPM_LOCAL_VERSION"

# Check if versions match
if [ "$CARGO_VERSION" != "$NPM_LOCAL_VERSION" ]; then
    echo "ERROR: Version mismatch!"
    echo "  Cargo.toml: $CARGO_VERSION"
    echo "  wasm/package.json: $NPM_LOCAL_VERSION"
    exit 1
fi

# Check for hardcoded old versions in test fixtures
echo ""
echo "Checking for hardcoded versions in tests/examples..."

OLD_VERSIONS=$(grep -r '"version":.*"0\.2\.[0-4]"' tests/ examples/ 2>/dev/null || true)

if [ -n "$OLD_VERSIONS" ]; then
    echo "WARNING: Found old version references (should be $CARGO_VERSION):"
    echo "$OLD_VERSIONS"
    echo ""
    echo "Update these to the current version: $CARGO_VERSION"
    exit 1
fi

# Check npm registry version (warning only)
echo ""
echo "Checking npm registry..."
NPM_REGISTRY_VERSION=$(npm view @d-o-hub/chaotic_semantic_memory version 2>/dev/null || echo "not published")

if [ "$NPM_REGISTRY_VERSION" = "not published" ]; then
    echo "⚠ WASM package not yet published to npm"
elif [ "$NPM_REGISTRY_VERSION" != "$CARGO_VERSION" ]; then
    echo "⚠ npm registry version ($NPM_REGISTRY_VERSION) differs from local ($CARGO_VERSION)"
    echo "  Run: cd wasm && npm publish"
else
    echo "✓ npm registry version: $NPM_REGISTRY_VERSION"
fi

echo ""
echo "✓ All local versions synchronized: $CARGO_VERSION"