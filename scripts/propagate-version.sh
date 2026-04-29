#!/usr/bin/env bash
# =============================================================================
# propagate-version.sh - Sync VERSION file to all version references
# =============================================================================
# Usage: scripts/propagate-version.sh [--dry-run] [--check]
#
# Reads version from VERSION file and propagates to:
#   - Cargo.toml
#   - wasm/package.json
#   - cli-npm/package.json
#   - CHANGELOG.md (unreleased section)
#   - README.md (version references)
#
# Flags:
#   --dry-run    Show changes without modifying files
#   --check      Verify versions are synced (exit 1 if mismatch)
#   --help       Show this help message
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION_FILE="$REPO_ROOT/VERSION"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; NC='\033[0m'

# Flags
DRY_RUN=false
CHECK_MODE=false

# =============================================================================
# Help
# =============================================================================
show_help() {
  cat << EOF
Usage: scripts/propagate-version.sh [flags]

Sync VERSION file to all version references in the project.

Flags:
  --dry-run    Show changes without modifying files
  --check      Verify versions are synced (exit 1 if mismatch)
  --help       Show this help message

Files updated:
  - Cargo.toml
  - wasm/package.json
  - cli-npm/package.json
  - CHANGELOG.md
  - README.md

Examples:
  scripts/propagate-version.sh              # Sync all files
  scripts/propagate-version.sh --dry-run    # Preview changes
  scripts/propagate-version.sh --check      # Verify sync
EOF
  exit 0
}

# =============================================================================
# Argument parsing
# =============================================================================
while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run)  DRY_RUN=true; shift ;;
    --check)    CHECK_MODE=true; shift ;;
    --help|-h)  show_help ;;
    *)          echo "Unknown flag: $1"; exit 1 ;;
  esac
done

# =============================================================================
# Validate VERSION file
# =============================================================================
if [[ ! -f "$VERSION_FILE" ]]; then
  echo -e "${RED}Error: VERSION file not found at $VERSION_FILE${NC}"
  exit 1
fi

VERSION="$(cat "$VERSION_FILE" | tr -d '[:space:]')"

# Validate semver format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo -e "${RED}Error: VERSION file contains invalid semver: $VERSION${NC}"
  echo "Expected format: X.Y.Z (e.g., 0.3.5)"
  exit 1
fi

echo -e "${CYAN}Version from VERSION file: ${VERSION}${NC}"

# =============================================================================
# Helper functions
# =============================================================================
get_cargo_version() {
  grep '^version = ' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

get_npm_version() {
  local file="$1"
  if [[ -f "$file" ]]; then
    grep '"version"' "$file" | head -1 | sed 's/.*"\([0-9][^"]*\)".*/\1/'
  else
    echo "none"
  fi
}

update_file() {
  local file="$1"
  local pattern="$2"
  local replacement="$3"

  if [[ ! -f "$file" ]]; then
    return
  fi

  if $DRY_RUN; then
    echo -e "  ${YELLOW}[dry-run]${NC} Would update $file"
  else
    sed -i "$pattern" "$file"
    echo -e "  ${GREEN}Updated:${NC} $file"
  fi
}

check_version_match() {
  local file="$1"
  local current="$2"
  local expected="$3"

  if [[ "$current" == "$expected" ]]; then
    echo -e "  ${GREEN}OK:${NC} $file ($current)"
    return 0
  else
    echo -e "  ${RED}MISMATCH:${NC} $file (has $current, expected $expected)"
    return 1
  fi
}

# =============================================================================
# Check mode: Verify versions are synced
# =============================================================================
if $CHECK_MODE; then
  echo -e "\n${CYAN}Checking version sync...${NC}"

  ERRORS=0

  # Cargo.toml
  CARGO_VER="$(get_cargo_version)"
  check_version_match "Cargo.toml" "$CARGO_VER" "$VERSION" || ((ERRORS++))

  # wasm/package.json
  WASM_VER="$(get_npm_version "$REPO_ROOT/wasm/package.json")"
  check_version_match "wasm/package.json" "$WASM_VER" "$VERSION" || ((ERRORS++))

  # cli-npm/package.json
  CLI_VER="$(get_npm_version "$REPO_ROOT/cli-npm/package.json")"
  check_version_match "cli-npm/package.json" "$CLI_VER" "$VERSION" || ((ERRORS++))

  # CHANGELOG.md - check for version header
  if [[ -f "$REPO_ROOT/CHANGELOG.md" ]]; then
    if grep -q "^## \[$VERSION\]" "$REPO_ROOT/CHANGELOG.md"; then
      echo -e "  ${GREEN}OK:${NC} CHANGELOG.md (has [$VERSION] section)"
    else
      echo -e "  ${YELLOW}WARN:${NC} CHANGELOG.md (missing [$VERSION] section - may be unreleased)"
    fi
  fi

  if [[ $ERRORS -gt 0 ]]; then
    echo -e "\n${RED}Version sync check failed with $ERRORS mismatches${NC}"
    echo "Run: scripts/propagate-version.sh to sync"
    exit 1
  fi

  echo -e "\n${GREEN}All versions are synced!${NC}"
  exit 0
fi

# =============================================================================
# Propagate mode: Update all files
# =============================================================================
echo -e "\n${CYAN}Propagating version $VERSION...${NC}"

CURRENT_CARGO="$(get_cargo_version)"

if [[ "$CURRENT_CARGO" == "$VERSION" ]]; then
  echo -e "  ${GREEN}Already synced:${NC} Cargo.toml"
else
  echo -e "  Updating Cargo.toml: $CURRENT_CARGO → $VERSION"
  update_file "$REPO_ROOT/Cargo.toml" "s/^version = \"$CURRENT_CARGO\"/version = \"$VERSION\"/"
fi

# wasm/package.json
WASM_VER="$(get_npm_version "$REPO_ROOT/wasm/package.json")"
if [[ "$WASM_VER" != "$VERSION" && "$WASM_VER" != "none" ]]; then
  echo -e "  Updating wasm/package.json: $WASM_VER → $VERSION"
  update_file "$REPO_ROOT/wasm/package.json" "s/\"version\": \"$WASM_VER\"/\"version\": \"$VERSION\"/"
fi

# cli-npm/package.json
CLI_VER="$(get_npm_version "$REPO_ROOT/cli-npm/package.json")"
if [[ "$CLI_VER" != "$VERSION" && "$CLI_VER" != "none" ]]; then
  echo -e "  Updating cli-npm/package.json: $CLI_VER → $VERSION"
  update_file "$REPO_ROOT/cli-npm/package.json" "s/\"version\": \"$CLI_VER\"/\"version\": \"$VERSION\"/"
fi

# CHANGELOG.md - convert [Unreleased] to version header
if [[ -f "$REPO_ROOT/CHANGELOG.md" ]]; then
  if grep -q "^## \[Unreleased\]" "$REPO_ROOT/CHANGELOG.md"; then
    TODAY="$(date -u +%Y-%m-%d)"
    echo -e "  Converting CHANGELOG [Unreleased] → [$VERSION]"
    if $DRY_RUN; then
      echo -e "    ${YELLOW}[dry-run]${NC} Would add: ## [$VERSION] - $TODAY"
    else
      # Add version header under Unreleased, keep Unreleased empty
      if ! grep -q "^## \[$VERSION\]" "$REPO_ROOT/CHANGELOG.md"; then
        sed -i "s/^## \[Unreleased\]/## [Unreleased]\n\n## [$VERSION] - $TODAY/" "$REPO_ROOT/CHANGELOG.md"
        echo -e "  ${GREEN}Updated:${NC} CHANGELOG.md"
      fi
    fi
  fi
fi

# README.md - update version references in code blocks
if [[ -f "$REPO_ROOT/README.md" ]]; then
  MAJOR_MINOR="$(echo "$VERSION" | cut -d. -f1,2)"
  if grep -q "chaotic_semantic_memory = { version = \"$MAJOR_MINOR\"" "$REPO_ROOT/README.md" ||
     grep -q "chaotic_semantic_memory = { version = \"$CURRENT_CARGO\"" "$REPO_ROOT/README.md"; then
    echo -e "  Updating README.md version references"
    update_file "$REPO_ROOT/README.md" "s/chaotic_semantic_memory = { version = \"[0-9]\.[0-9][^\"]*\"/chaotic_semantic_memory = { version = \"$MAJOR_MINOR\"/" "g"
  fi
fi

# Update Cargo.lock
if ! $DRY_RUN; then
  echo -e "  Updating Cargo.lock..."
  cargo check --quiet 2>/dev/null || true
fi

echo -e "\n${GREEN}Version propagation complete!${NC}"
echo "Version: $VERSION"