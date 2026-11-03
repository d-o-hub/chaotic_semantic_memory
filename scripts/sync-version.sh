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
    echo "  Would update workspace member versions (crates/ + benchmarks)"
    echo "  Would update every intra-workspace dependency pin in every manifest (root -> member and member -> anything)"
    exit 0
fi

# 1. Update VERSION file
echo "$VERSION" > VERSION
echo "  ✓ Updated VERSION"

# 2. Update Cargo.toml root package version
# Anchored on any semver, not on $CURRENT_VERSION: the old conditional silently skipped
# the rewrite when the root `version =` line was not the first `^version = ` hit, and
# every later step then synced edges against a package that had not moved.
sed -i -E 's/^version = "[0-9]+\.[0-9]+\.[0-9]+"/version = "'"$VERSION"'"/' Cargo.toml
echo "  ✓ Updated Cargo.toml root version"

# 3. Update documentation / examples version strings
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

# 4. Update workspace member versions
# `crates/` alone missed `benchmarks`, which the root [workspace] lists as a member:
# measured on 5b04e9c8 it sat three releases behind at 0.3.6 while every crate was 0.3.9,
# and verify-version-sync.sh's identical glob could not see it either.
echo "  Updating workspace crates..."
find crates benchmarks -name "Cargo.toml" -exec sed -i -E 's/^version = "[0-9]+\.[0-9]+\.[0-9]+"/version = "'"$VERSION"'"/g' {} +
echo "  ✓ Updated workspace crate versions"

# 5. Update every intra-workspace dependency pin, in every manifest and every section.
# This replaces the root-only sed that matched `path = "crates/csm-…", version = "…"`:
# one direction of one file, which is how 18 member→anything edges drifted to 0.3.8 and
# 0.3.7 unnoticed (issue #964). Runs after package versions so the tool compares each pin
# against the version the target actually has now. The tool rewrites pins inside dependency
# tables only, never a `[package] version`, which is why fuzz's deliberate `0.0.0` survives
# it while fuzz's multi-line `[dependencies.chaotic_semantic_memory]` pin does not.
if ! python3 scripts/workspace-version-edges.py --sync "$VERSION"; then
    # Steps 1-4 have already rewritten VERSION, Cargo.toml and the member manifests, so a
    # silent death here leaves a half-synced tree; say that out loud instead of letting the
    # caller read "set -e" as "nothing happened".
    echo "Error: the pin sync refused (see the message above). The tree is HALF-synced:" >&2
    echo "  VERSION, Cargo.toml, wasm/, cli-npm/ and the member versions were already rewritten." >&2
    exit 2
fi
echo "  ✓ Updated intra-workspace dependency pins"

# 6. Synchronize Cargo.lock AFTER manifest updates
echo "  Updating Cargo.lock..."
cargo check --workspace >/dev/null 2>&1 || cargo update
echo "  ✓ Synchronized Cargo.lock"

# 7. Generate llms.txt files
if [ -f "scripts/gen-llms-txt.sh" ]; then
    bash scripts/gen-llms-txt.sh || echo "Warning: llms.txt generation failed"
fi

# 8. Fail if a docs/tests/examples literal is still not the target version.
# Steps 3 replaces only $CURRENT_VERSION, so a literal that was already older
# than the pre-release manifest was invisible (issue #982). This enumerator reads
# the live version and accepts only current values or an explicit
# `version-fixture:` annotation, so the sweep cannot report success over drift.
if [ -f "scripts/check-version-literals.py" ]; then
    if ! python3 scripts/check-version-literals.py .; then
        echo "Error: version literals in tests/, examples/ or docs/ are not $VERSION." >&2
        echo "  Update them, or annotate a deliberately-older fixture with 'version-fixture:'." >&2
        exit 2
    fi
else
    echo "Error: scripts/check-version-literals.py is missing; the literal gate cannot run." >&2
    exit 2
fi

echo "Version sync complete: target version is $VERSION"
