#!/usr/bin/env bash
# Create a GitHub release with auto-generated release notes
# Usage: ./create-github-release.sh v1.2.0

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$REPO_ROOT"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
info() { echo -e "${BLUE}ℹ${NC} $1"; }

# Parse arguments
TAG="${1:-}"
if [[ -z "$TAG" ]]; then
    echo "Usage: $0 <tag>"
    echo "  Example: $0 v1.2.0"
    exit 1
fi

# Normalize tag (add v prefix if missing)
if [[ ! "$TAG" =~ ^v ]]; then
    TAG="v$TAG"
fi

VERSION="${TAG#v}"
echo -e "${BLUE}=== Creating GitHub Release ===${NC}"
echo "Tag: $TAG"
echo "Version: $VERSION"
echo ""

# Check gh CLI is available
if ! command -v gh &> /dev/null; then
    fail "GitHub CLI (gh) not found. Install: https://cli.github.com/"
fi

# Check authentication
if ! gh auth status &> /dev/null; then
    fail "Not authenticated with GitHub. Run: gh auth login"
fi
pass "GitHub CLI authenticated"

# Verify tag format
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    fail "Invalid tag format. Expected: v1.2.3 or v1.2.3-beta.1"
fi
pass "Tag format valid"

# Check version matches Cargo.toml
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
if [[ "$VERSION" != "$CARGO_VERSION" ]]; then
    fail "Version mismatch: tag=$VERSION, Cargo.toml=$CARGO_VERSION"
fi
pass "Version matches Cargo.toml"

# Check if tag exists locally
if git tag -l | grep -q "^$TAG$"; then
    fail "Tag $TAG already exists locally. Delete it first: git tag -d $TAG"
fi
pass "Tag does not exist locally"

# Check if tag exists on remote
if git ls-remote --tags origin | grep -q "refs/tags/$TAG$"; then
    fail "Tag $TAG already exists on remote"
fi
pass "Tag does not exist on remote"

# Get previous tag for release notes
PREV_TAG=$(git describe --tags --abbrev=0 HEAD 2>/dev/null || echo "")
if [[ -n "$PREV_TAG" ]]; then
    info "Previous tag: $PREV_TAG"
else
    info "No previous tag found (first release?)"
fi

# Generate release notes
echo ""
echo "Generating release notes..."

if [[ -n "$PREV_TAG" ]]; then
    # Get commits since last tag
    COMMITS=$(git log --pretty=format:"- %s" "$PREV_TAG"..HEAD 2>/dev/null || echo "")
else
    # Get all commits
    COMMITS=$(git log --pretty=format:"- %s" HEAD~10..HEAD 2>/dev/null || echo "")
fi

# Count commits by type
FEAT_COUNT=$(echo "$COMMITS" | grep -c "feat" || echo "0")
FIX_COUNT=$(echo "$COMMITS" | grep -c "fix" || echo "0")
PERF_COUNT=$(echo "$COMMITS" | grep -c "perf" || echo "0")
BREAKING_COUNT=$(echo "$COMMITS" | grep -c "BREAKING\|!:" || echo "0")

echo ""
echo "Changes since $PREV_TAG:"
echo "  Features: $FEAT_COUNT"
echo "  Fixes:    $FIX_COUNT"
echo "  Perf:     $PERF_COUNT"
echo "  Breaking: $BREAKING_COUNT"
echo ""

# Build release notes
RELEASE_NOTES="## Release $VERSION

### Changes

$COMMITS

### Installation

\`\`\`bash
# Add to Cargo.toml
[dependencies]
chaotic_semantic_memory = \"$VERSION\"

# Or use cargo add
cargo add chaotic_semantic_memory@$VERSION
\`\`\`

---

[View full changelog](https://github.com/$(git remote get-url origin | sed 's/.*github.com[\/:]\(.*\)\.git/\1/')/compare/${PREV_TAG}...${TAG})"

# Confirm release
echo "================================"
echo "Release Notes Preview:"
echo "================================"
echo "$RELEASE_NOTES"
echo "================================"
echo ""

read -p "Create release $TAG? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Create annotated tag
echo ""
echo "Creating tag $TAG..."
git tag -a "$TAG" -m "Release $VERSION"
pass "Tag created"

# Push tag to origin
echo ""
echo "Pushing tag to origin..."
git push origin "$TAG"
pass "Tag pushed"

# Create GitHub release
echo ""
echo "Creating GitHub release..."
gh release create "$TAG" \
    --title "Release $VERSION" \
    --notes "$RELEASE_NOTES" \
    --verify-tag

pass "GitHub release created"

echo ""
echo -e "${GREEN}=== Release $TAG created successfully! ===${NC}"
echo ""
echo "Next steps:"
echo "  1. Monitor CI: gh run watch"
echo "  2. Verify crates.io: cargo search chaotic_semantic_memory"
echo "  3. View release: gh release view $TAG"
echo ""
