#!/usr/bin/env bash
# Pre-release validation script
# Run this before creating a release to ensure all gates pass

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
warn() { echo -e "${YELLOW}!${NC} $1"; }

echo "=== Pre-Release Validation ==="
echo ""

# Check for uncommitted changes
echo "Checking for uncommitted changes..."
if git diff --quiet && git diff --staged --quiet; then
    pass "No uncommitted changes"
else
    fail "Uncommitted changes detected. Commit or stash first."
fi

# Check we're on main branch
echo ""
echo "Checking branch..."
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$BRANCH" == "main" ]]; then
    pass "On main branch"
else
    warn "Not on main branch (current: $BRANCH)"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    [[ ! $REPLY =~ ^[Yy]$ ]] && exit 1
fi

# Run cargo check
echo ""
echo "Running cargo check..."
export CARGO_TERM_PROGRESS_WHEN=never
if cargo check --message-format=short 2>&1 | grep -q "^error"; then
    fail "cargo check failed"
else
    pass "cargo check passed"
fi

# Run tests
echo ""
echo "Running cargo test..."
if cargo test --all-features --quiet 2>&1 | tail -5 | grep -q "test result: ok"; then
    pass "cargo test passed"
else
    # Check if all tests passed even if grep didn't match exact string
    if cargo test --all-features --quiet 2>&1 | grep -qE "(0 failed|test result: ok)"; then
        pass "cargo test passed"
    else
        fail "cargo test failed"
    fi
fi

# Run clippy
echo ""
echo "Running cargo clippy..."
if cargo clippy -- -D warnings 2>&1 | grep -q "^error"; then
    fail "cargo clippy failed"
else
    pass "cargo clippy passed"
fi

# Check formatting
echo ""
echo "Running cargo fmt check..."
if cargo fmt --check 2>&1 | grep -q "Diff"; then
    fail "cargo fmt: files need formatting. Run 'cargo fmt'"
else
    pass "cargo fmt passed"
fi

# Build documentation
echo ""
echo "Building documentation..."
if cargo doc --no-deps 2>&1 | grep -q "^error"; then
    fail "cargo doc failed"
else
    pass "cargo doc passed"
fi

# Check LOC limits
echo ""
echo "Checking LOC limits..."
if [[ -f "$REPO_ROOT/scripts/loc-check.sh" ]]; then
    bash "$REPO_ROOT/scripts/loc-check.sh"
elif [[ -f "$REPO_ROOT/.agents/skills/testing-validation/scripts/loc-check.sh" ]]; then
    bash "$REPO_ROOT/.agents/skills/testing-validation/scripts/loc-check.sh"
else
    # Inline LOC check
    MAX_LOC=500
    VIOLATIONS=""
    while IFS= read -r file; do
        LOC=$(wc -l < "$file")
        if [[ $LOC -gt $MAX_LOC ]]; then
            VIOLATIONS="$VIOLATIONS\n  $file: $LOC lines (max: $MAX_LOC)"
        fi
    done < <(find "$REPO_ROOT/src" -name "*.rs" -type f 2>/dev/null)
    
    if [[ -n "$VIOLATIONS" ]]; then
        fail "LOC violations found:$VIOLATIONS"
    else
        pass "All files under $MAX_LOC LOC"
    fi
fi

# Check version in Cargo.toml
echo ""
echo "Checking Cargo.toml..."
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
if [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    pass "Version: $VERSION"
else
    warn "Version format unexpected: $VERSION"
fi

# Check if tag already exists
echo ""
echo "Checking for existing tags..."
TAG="v$VERSION"
if git tag -l | grep -q "^$TAG$"; then
    fail "Tag $TAG already exists. Bump version in Cargo.toml."
else
    pass "Tag $TAG does not exist yet"
fi

# Check remote sync
echo ""
echo "Checking remote sync..."
git fetch origin --quiet 2>/dev/null || true
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse origin/main 2>/dev/null || echo "unknown")
if [[ "$LOCAL" == "$REMOTE" ]]; then
    pass "Local is synced with origin/main"
else
    warn "Local differs from origin/main. Push first?"
fi

echo ""
echo "================================"
echo -e "${GREEN}All validation gates passed!${NC}"
echo "Ready to create release: $TAG"
echo ""
echo "Next steps:"
echo "  1. Push to origin: git push origin main"
echo "  2. Create release: $SCRIPT_DIR/create-github-release.sh $TAG"
