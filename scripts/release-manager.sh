#!/usr/bin/env bash
# =============================================================================
# Release Manager - Unified release automation for chaotic_semantic_memory
# ADR-0042: Replaces validate-release.sh + create-github-release.sh
#
# Usage:
#   scripts/release-manager.sh validate              # Pre-release checks
#   scripts/release-manager.sh prepare <version>      # Bump + changelog + sync
#   scripts/release-manager.sh publish <version>      # Tag + push + release
#   scripts/release-manager.sh rollback <version>     # Undo a failed release
#   scripts/release-manager.sh full <version>         # validate + prepare + publish
#
# Flags:
#   --yes           Non-interactive mode (CI-safe)
#   --dry-run       Simulate without side effects
#   --log <file>    Write structured log to file
# =============================================================================
set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
readonly LOG_PREFIX="[release-manager]"

# --- Colors & output --------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

# --- Flags -------------------------------------------------------------------
YES_MODE=false
DRY_RUN=false
LOG_FILE=""

# --- Counters ----------------------------------------------------------------
PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

# --- Logging -----------------------------------------------------------------
log_info()  { echo -e "${BLUE}ℹ${NC} ${LOG_PREFIX} $1"; _log "INFO" "$1"; }
log_pass()  { echo -e "${GREEN}✓${NC} ${LOG_PREFIX} $1"; _log "PASS" "$1"; ((PASS_COUNT++)) || true; }
log_warn()  { echo -e "${YELLOW}⚠${NC} ${LOG_PREFIX} $1"; _log "WARN" "$1"; ((WARN_COUNT++)) || true; }
log_fail()  { echo -e "${RED}✗${NC} ${LOG_PREFIX} $1"; _log "FAIL" "$1"; ((FAIL_COUNT++)) || true; }
log_step()  { echo -e "\n${CYAN}━━━ $1 ━━━${NC}"; _log "STEP" "$1"; }
log_fatal() { log_fail "$1"; _summary; exit 1; }

_log() {
  if [[ -n "$LOG_FILE" ]]; then
    echo "{\"ts\":\"$TIMESTAMP\",\"level\":\"$1\",\"msg\":\"$2\"}" >> "$LOG_FILE"
  fi
}

_summary() {
  echo -e "\n${CYAN}━━━ Summary ━━━${NC}"
  echo -e "  ${GREEN}Passed:${NC}   $PASS_COUNT"
  echo -e "  ${YELLOW}Warnings:${NC} $WARN_COUNT"
  echo -e "  ${RED}Failed:${NC}   $FAIL_COUNT"
}

# --- Helpers -----------------------------------------------------------------
confirm() {
  if $YES_MODE; then return 0; fi
  read -p "$1 (y/N) " -n 1 -r
  echo
  [[ $REPLY =~ ^[Yy]$ ]]
}

run_or_dry() {
  if $DRY_RUN; then
    log_info "[dry-run] Would execute: $*"
  else
    "$@"
  fi
}

cargo_version() {
  grep '^version = ' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
}

npm_version() {
  if [[ -f "$REPO_ROOT/wasm/package.json" ]]; then
    grep '"version"' "$REPO_ROOT/wasm/package.json" | head -1 | sed 's/.*"\([0-9][^"]*\)".*/\1/'
  else
    echo "none"
  fi
}

# =============================================================================
# VALIDATE: Pre-release checks (exit-code based, no grep hacks)
# =============================================================================
cmd_validate() {
  log_step "Pre-Release Validation"
  cd "$REPO_ROOT"

  # 1. Clean workspace
  log_step "Workspace cleanliness"
  if git diff --quiet && git diff --staged --quiet; then
    log_pass "No uncommitted changes"
  else
    log_fail "Uncommitted changes detected"
    git diff --stat
    git diff --staged --stat
    if ! confirm "Continue with dirty workspace?"; then
      log_fatal "Aborted: clean workspace first"
    fi
  fi

  # 2. Branch check
  local branch
  branch="$(git rev-parse --abbrev-ref HEAD)"
  if [[ "$branch" == "main" ]]; then
    log_pass "On main branch"
  else
    log_warn "Not on main branch (current: $branch)"
  fi

  # 3. Cargo check (exit code, not grep)
  log_step "Cargo gates"
  if cargo check --all-targets --all-features --quiet 2>&1; then
    log_pass "cargo check passed"
  else
    log_fatal "cargo check failed"
  fi

  # 4. Formatting
  if cargo fmt --check --quiet 2>&1; then
    log_pass "cargo fmt passed"
  else
    log_fatal "cargo fmt failed — run 'cargo fmt'"
  fi

  # 5. Clippy
  if cargo clippy --all-targets --all-features -- -D warnings 2>&1; then
    log_pass "cargo clippy passed"
  else
    log_fatal "cargo clippy failed"
  fi

  # 6. Tests (single run, not double)
  if cargo test --all-features --quiet 2>&1; then
    log_pass "cargo test passed"
  else
    log_fatal "cargo test failed"
  fi

  # 7. Doc build
  if cargo doc --no-deps --quiet 2>&1; then
    log_pass "cargo doc passed"
  else
    log_fatal "cargo doc failed"
  fi

  # 8. Cargo publish dry-run (critical gate)
  log_step "Publish dry-run"
  if cargo publish --dry-run --allow-dirty 2>&1; then
    log_pass "cargo publish --dry-run passed"
  else
    log_fail "cargo publish --dry-run failed"
    log_info "Fix packaging issues before release"
  fi

  # 9. LOC limits
  log_step "LOC limits"
  local loc_ok=true
  while IFS= read -r file; do
    local loc
    loc="$(wc -l < "$file")"
    if [[ "$loc" -gt 500 ]]; then
      log_fail "$file exceeds 500 LOC ($loc lines)"
      loc_ok=false
    fi
  done < <(find "$REPO_ROOT/src" -name '*.rs' -type f)
  if $loc_ok; then
    log_pass "All source files under 500 LOC"
  fi

  # 10. WASM target
  if rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    if cargo check --target wasm32-unknown-unknown --features wasm --quiet 2>&1; then
      log_pass "WASM target compiles"
    else
      log_warn "WASM target check failed"
    fi
  else
    log_warn "WASM target not installed, skipping"
  fi

  # 11. Security: cargo audit (if available)
  if command -v cargo-audit &> /dev/null; then
    if cargo audit --quiet 2>&1; then
      log_pass "cargo audit passed (no known vulnerabilities)"
    else
      log_warn "cargo audit found issues"
    fi
  else
    log_warn "cargo-audit not installed, skipping vulnerability check"
  fi

  # 12. Version info
  log_step "Version check"
  local version
  version="$(cargo_version)"
  log_info "Cargo.toml version: $version"
  log_info "wasm/package.json version: $(npm_version)"

  if git tag -l | grep -q "^v${version}$"; then
    log_warn "Tag v$version already exists"
  else
    log_pass "Tag v$version is available"
  fi

  _summary

  if [[ "$FAIL_COUNT" -gt 0 ]]; then
    log_fatal "Validation failed with $FAIL_COUNT errors"
  fi

  echo -e "\n${GREEN}All validation gates passed!${NC}"
  echo "Ready to release: v$version"
}

# =============================================================================
# PREPARE: Version bump + changelog + sync
# =============================================================================
cmd_prepare() {
  local target_version="${1:-}"
  if [[ -z "$target_version" ]]; then
    echo "Usage: $0 prepare <version>"
    echo "  Example: $0 prepare 0.1.0"
    exit 1
  fi

  # Strip v prefix if present
  target_version="${target_version#v}"
  log_step "Preparing release v$target_version"
  cd "$REPO_ROOT"

  # 1. Update Cargo.toml version
  local current_version
  current_version="$(cargo_version)"
  if [[ "$current_version" != "$target_version" ]]; then
    log_info "Bumping Cargo.toml: $current_version → $target_version"
    run_or_dry sed -i "s/^version = \"$current_version\"/version = \"$target_version\"/" Cargo.toml
    log_pass "Cargo.toml version updated"
  else
    log_pass "Cargo.toml already at $target_version"
  fi

  # 2. Sync wasm/package.json version
  if [[ -f "$REPO_ROOT/wasm/package.json" ]]; then
    local npm_ver
    npm_ver="$(npm_version)"
    if [[ "$npm_ver" != "$target_version" ]]; then
      log_info "Syncing wasm/package.json: $npm_ver → $target_version"
      run_or_dry sed -i "s/\"version\": \"$npm_ver\"/\"version\": \"$target_version\"/" wasm/package.json
      log_pass "wasm/package.json version synced"
    else
      log_pass "wasm/package.json already at $target_version"
    fi
  fi

  # 3. Merge CHANGELOG [Unreleased] → [version]
  if [[ -f CHANGELOG.md ]]; then
    local today
    today="$(date -u +%Y-%m-%d)"
    if grep -q '## \[Unreleased\]' CHANGELOG.md; then
      # Check if there's content under [Unreleased]
      local unreleased_content
      unreleased_content="$(sed -n '/## \[Unreleased\]/,/## \[/p' CHANGELOG.md | sed '1d;$d')"
      if [[ -n "$unreleased_content" ]]; then
        log_info "Merging [Unreleased] content into [$target_version]"
        # Check if version section already exists
        if grep -q "## \[$target_version\]" CHANGELOG.md; then
          # Merge unreleased content into existing version section
          run_or_dry sed -i "/## \[Unreleased\]/,/## \[/{/## \[Unreleased\]/!{/## \[/!d}}" CHANGELOG.md
          run_or_dry sed -i "s/## \[Unreleased\]/## [Unreleased]\n/" CHANGELOG.md
        else
          # Replace [Unreleased] header, keep content, add version header
          run_or_dry sed -i "s/## \[Unreleased\]/## [Unreleased]\n\n## [$target_version] - $today/" CHANGELOG.md
        fi
        log_pass "CHANGELOG.md updated for v$target_version"
      else
        log_info "No unreleased changes to merge"
      fi
    else
      log_warn "No [Unreleased] section found in CHANGELOG.md"
    fi

    # Update comparison links at bottom
    local repo_url="https://github.com/d-o-hub/chaotic_semantic_memory"
    if ! grep -q "\[${target_version}\]:" CHANGELOG.md; then
      run_or_dry sed -i "/^\[unreleased\]:/a [$target_version]: $repo_url/releases/tag/v$target_version" CHANGELOG.md
    fi
    run_or_dry sed -i "s|\[unreleased\]:.*|[unreleased]: $repo_url/compare/v${target_version}...HEAD|" CHANGELOG.md
  fi

  # 4. Update Cargo.lock
  log_info "Updating Cargo.lock..."
  run_or_dry cargo check --quiet 2>/dev/null || true
  log_pass "Cargo.lock updated"

  # 5. Sync README.md version references
  if [[ -f "$REPO_ROOT/README.md" ]]; then
    log_info "Syncing README.md version references..."
    run_or_dry sed -i "s/| Version | \`[0-9][^\`]*\` |/| Version | \`${target_version}\` |/" README.md
    run_or_dry sed -i "s/chaotic_semantic_memory = { version = \"[0-9][^\"]*\"/chaotic_semantic_memory = { version = \"$target_version\"/" README.md
    log_pass "README.md version synced"
  fi

  # 6. Sync SECURITY.md supported versions
  if [[ -f "$REPO_ROOT/SECURITY.md" ]]; then
    log_info "Updating SECURITY.md supported versions..."
    if ! grep -q "| $target_version |" SECURITY.md; then
      run_or_dry sed -i "/^| Version/a | $target_version   | :white_check_mark: |" SECURITY.md
    fi
    log_pass "SECURITY.md updated"
  fi

  # 7. Sync book/src/getting-started.md version
  if [[ -f "$REPO_ROOT/book/src/getting-started.md" ]]; then
    log_info "Syncing book/src/getting-started.md..."
    run_or_dry sed -i "s/chaotic_semantic_memory = { version = \"[0-9][^\"]*\"/chaotic_semantic_memory = { version = \"$target_version\"/" book/src/getting-started.md
    log_pass "book/src/getting-started.md version synced"
  fi

  # 8. Sync wasm/README.md version references
  if [[ -f "$REPO_ROOT/wasm/README.md" ]]; then
    log_info "Syncing wasm/README.md..."
    run_or_dry sed -i "s/chaotic-semantic-memory@[0-9][^\"']*/chaotic-semantic-memory@$target_version/" wasm/README.md 2>/dev/null || true
    log_pass "wasm/README.md synced"
  fi

  # 9. Regenerate llms.txt files (SKIPPED - runs in GitHub workflows)
  log_info "llms.txt: skipping local generation (handled by CI)"

  # 10. Sync AGENTS.md if it contains version references
  if [[ -f "$REPO_ROOT/AGENTS.md" ]]; then
    if grep -q "Version.*0\." AGENTS.md 2>/dev/null; then
      log_info "Syncing AGENTS.md version references..."
      run_or_dry sed -i "s/Version.*\`[0-9][^\`]*\`/Version \`${target_version}\`/" AGENTS.md 2>/dev/null || true
      log_pass "AGENTS.md synced"
    fi
  fi

  _summary
  echo -e "\n${GREEN}Release v$target_version prepared!${NC}"
  echo "Review changes, then run: $0 publish $target_version"
}

# =============================================================================
# PUBLISH: Tag + push + create GitHub release
# =============================================================================
cmd_publish() {
  local target_version="${1:-}"
  if [[ -z "$target_version" ]]; then
    echo "Usage: $0 publish <version>"
    exit 1
  fi
  target_version="${target_version#v}"
  local tag="v$target_version"

  log_step "Publishing release $tag"
  cd "$REPO_ROOT"

  # Pre-flight checks
  if ! command -v gh &> /dev/null; then
    log_fatal "GitHub CLI (gh) not found. Install: https://cli.github.com/"
  fi

  if ! gh auth status &> /dev/null 2>&1; then
    log_fatal "Not authenticated with GitHub. Run: gh auth login"
  fi
  log_pass "GitHub CLI authenticated"

  # Verify version matches
  local cargo_ver
  cargo_ver="$(cargo_version)"
  if [[ "$cargo_ver" != "$target_version" ]]; then
    log_fatal "Version mismatch: Cargo.toml=$cargo_ver, requested=$target_version. Run 'prepare' first."
  fi
  log_pass "Version matches Cargo.toml: $target_version"

  # Check tag doesn't exist
  if git tag -l | grep -q "^$tag$"; then
    log_fatal "Tag $tag already exists locally. Delete first: git tag -d $tag"
  fi
  if git ls-remote --tags origin 2>/dev/null | grep -q "refs/tags/$tag$"; then
    log_fatal "Tag $tag already exists on remote"
  fi
  log_pass "Tag $tag is available"

  # Final cargo publish dry-run
  log_step "Final publish dry-run"
  if ! cargo publish --dry-run 2>&1; then
    log_fatal "cargo publish --dry-run failed. Fix packaging issues first."
  fi
  log_pass "Publish dry-run passed"

  # Confirm
  if ! confirm "Create and push tag $tag? This triggers CI publishing."; then
    log_info "Aborted by user"
    exit 0
  fi

  # Create annotated tag
  log_info "Creating annotated tag $tag..."
  run_or_dry git tag -a "$tag" -m "Release $target_version"
  log_pass "Tag $tag created"

  # Push tag (this triggers release.yml)
  log_info "Pushing tag to origin..."
  run_or_dry git push origin "$tag"
  log_pass "Tag $tag pushed — CI release workflow triggered"

  # Extract release notes from CHANGELOG
  local release_notes=""
  if [[ -f CHANGELOG.md ]]; then
    release_notes="$(awk "/^## \\[$target_version\\]/,/^## \\[/" CHANGELOG.md | sed '1d;$d')"
  fi
  if [[ -z "$release_notes" ]]; then
    release_notes="Release $target_version"
  fi

  # Create GitHub Release (with notes, CI adds artifacts)
  log_info "Creating GitHub Release..."
  if ! $DRY_RUN; then
    gh release create "$tag" \
      --title "chaotic_semantic_memory v$target_version" \
      --notes "$release_notes" \
      --verify-tag
  fi
  log_pass "GitHub Release created"

  _summary
  echo -e "\n${GREEN}━━━ Release $tag published! ━━━${NC}"
  echo ""
  echo "Next steps:"
  echo "  1. Monitor CI:         gh run watch"
  echo "  2. Verify crates.io:   cargo search chaotic_semantic_memory"
  echo "  3. View release:       gh release view $tag"
  echo "  4. If issues arise:    $0 rollback $target_version"
}

# =============================================================================
# ROLLBACK: Undo a failed/bad release
# =============================================================================
cmd_rollback() {
  local target_version="${1:-}"
  if [[ -z "$target_version" ]]; then
    echo "Usage: $0 rollback <version>"
    exit 1
  fi
  target_version="${target_version#v}"
  local tag="v$target_version"

  log_step "Rolling back release $tag"
  cd "$REPO_ROOT"

  if ! confirm "This will delete tag $tag and GitHub release. Continue?"; then
    exit 0
  fi

  # Delete GitHub release
  if gh release view "$tag" &> /dev/null 2>&1; then
    run_or_dry gh release delete "$tag" --yes
    log_pass "GitHub Release $tag deleted"
  else
    log_warn "No GitHub Release found for $tag"
  fi

  # Delete remote tag
  if git ls-remote --tags origin | grep -q "refs/tags/$tag$"; then
    run_or_dry git push --delete origin "$tag"
    log_pass "Remote tag $tag deleted"
  else
    log_warn "No remote tag $tag found"
  fi

  # Delete local tag
  if git tag -l | grep -q "^$tag$"; then
    run_or_dry git tag -d "$tag"
    log_pass "Local tag $tag deleted"
  fi

  # Yank from crates.io (if published)
  echo ""
  log_warn "If already published to crates.io, yank manually:"
  echo "  cargo yank --version $target_version chaotic_semantic_memory"

  _summary
  echo -e "\n${YELLOW}Rollback complete for $tag${NC}"
}

# =============================================================================
# FULL: validate + prepare + publish in one go
# =============================================================================
cmd_full() {
  local target_version="${1:-}"
  if [[ -z "$target_version" ]]; then
    echo "Usage: $0 full <version>"
    exit 1
  fi

  cmd_validate
  cmd_prepare "$target_version"
  cmd_publish "$target_version"
}

# =============================================================================
# Argument parsing
# =============================================================================
COMMAND=""
VERSION_ARG=""
POSITIONAL_ARGS=()

while [[ $# -gt 0 ]]; do
  case $1 in
    --yes|-y)      YES_MODE=true; shift ;;
    --dry-run|-n)  DRY_RUN=true; shift ;;
    --log)         LOG_FILE="$2"; shift 2 ;;
    -*)            echo "Unknown flag: $1"; exit 1 ;;
    *)             POSITIONAL_ARGS+=("$1"); shift ;;
  esac
done

COMMAND="${POSITIONAL_ARGS[0]:-}"
VERSION_ARG="${POSITIONAL_ARGS[1]:-}"

# Banner
echo -e "${CYAN}╔══════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║  chaotic_semantic_memory Release Manager     ║${NC}"
echo -e "${CYAN}║  $(date -u +%Y-%m-%d)                                  ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════╝${NC}"
echo ""

if $DRY_RUN; then log_info "Running in DRY-RUN mode"; fi
if $YES_MODE; then log_info "Running in non-interactive mode"; fi
if [[ -n "$LOG_FILE" ]]; then
  log_info "Logging to: $LOG_FILE"
  echo "{\"ts\":\"$TIMESTAMP\",\"event\":\"start\",\"command\":\"$COMMAND\",\"version\":\"$VERSION_ARG\"}" > "$LOG_FILE"
fi

case "$COMMAND" in
  validate)  cmd_validate ;;
  prepare)   cmd_prepare "$VERSION_ARG" ;;
  publish)   cmd_publish "$VERSION_ARG" ;;
  rollback)  cmd_rollback "$VERSION_ARG" ;;
  full)      cmd_full "$VERSION_ARG" ;;
  *)
    echo "Usage: $0 <command> [version] [flags]"
    echo ""
    echo "Commands:"
    echo "  validate              Run all pre-release validation gates"
    echo "  prepare <version>     Bump version, sync files, update changelog"
    echo "  publish <version>     Create tag, push, create GitHub release"
    echo "  rollback <version>    Undo a failed release (delete tag + release)"
    echo "  full <version>        Run validate + prepare + publish"
    echo ""
    echo "Flags:"
    echo "  --yes, -y             Non-interactive mode (skip confirmations)"
    echo "  --dry-run, -n         Simulate without side effects"
    echo "  --log <file>          Write structured JSON log to file"
    echo ""
    echo "Examples:"
    echo "  $0 validate"
    echo "  $0 full 0.1.0 --yes --log release.log"
    echo "  $0 rollback 0.1.0"
    exit 1
    ;;
esac
