#!/bin/bash
# =============================================================================
# sync-version.sh - Automate version updates for releases
# =============================================================================
# Usage: ./scripts/sync-version.sh <version> [--dry-run]
# Example: ./scripts/sync-version.sh 0.2.0
# =============================================================================
#
# ⚠️  IMPORTANT: Git tags are created automatically by GitHub Actions
#    DO NOT create tags manually! The release workflow will create them.
#
# Workflow:
#   1. Update version in Cargo.toml
#   2. Run: ./scripts/sync-version.sh <version>
#   3. Commit and push to main
#   4. GitHub Actions creates tag and releases automatically
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

# Guardrail: Validate CHANGELOG format before modifying
validate_changelog() {
  local ver="$1"
  local changelog="CHANGELOG.md"
  
  if [ ! -f "$changelog" ]; then
    echo "Error: $changelog not found"
    exit 1
  fi
  
  # Check for Unreleased section
  if ! grep -q "## \\[Unreleased\\]" "$changelog"; then
    echo "Error: Missing [Unreleased] section in $changelog"
    exit 1
  fi
  
  # Check for duplicate version headers (would break release workflow)
  local existing_count
  existing_count=$(grep -c "^## \[${ver}\]" "$changelog" || true)
  if [ "${existing_count:-0}" -gt 0 ]; then
    echo "Error: Version ${ver} already has ${existing_count} header(s) in $changelog"
    echo "  This would create duplicates. Remove existing headers for ${ver} first."
    exit 1
  fi
  
  # Check for version link entry at bottom
  if ! grep -q "^\\[${ver}\\]:" "$changelog" 2>/dev/null; then
    echo "Warning: Missing version link [${ver}]: at bottom of $changelog"
    echo "  This will be added automatically by sync-version"
  fi
}

# Get current version from Cargo.toml first
CURRENT_VERSION=$(grep -m1 'version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

# If version unchanged, skip validation and exit early
if [ "$VERSION" = "$CURRENT_VERSION" ]; then
    echo "Version $VERSION is current. No sync needed."
    exit 0
fi

# Run validation (only for version bumps)
validate_changelog "$VERSION"

# Extract major.minor for Cargo.toml compatibility version
MAJOR_MINOR=$(echo "$VERSION" | cut -d. -f1,2)

echo "=============================================="
echo "Version Sync: $VERSION"
echo "=============================================="

echo "Current version: $CURRENT_VERSION"
echo ""

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
VERSION_FILES["cli-npm/package.json"]="s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$VERSION\"/"

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
echo "⚠️  IMPORTANT: Git tags are now created automatically by GitHub Actions"
echo "    Do NOT create tags manually - they will be created on push to main"
echo ""
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Run validation: ./scripts/validate.sh"
echo "  3. Commit: git add -A && git commit -m \"release: v$VERSION\""
echo "  4. Push: git push origin main"
echo ""
echo "GitHub Actions will automatically:"
echo "  - Create tag v$VERSION"
echo "  - Publish to crates.io"
echo "  - Publish to npm"
echo "  - Create GitHub Release"
