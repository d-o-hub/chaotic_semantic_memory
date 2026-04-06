---
name: release-management
description: GitHub release management, crates.io trusted publishing, npm provenance, and GitHub Pages documentation. Use when creating releases, publishing packages, or deploying docs.
---

# Release Management

Automated release pipeline using 2026 best practices: version sync automation, Trusted Publishing for crates.io/npm, and mdBook for docs.

## Quick Start

```bash
# 1. Sync version across all files (automates README, docs, tests)
./scripts/sync-version.sh 0.2.0

# 2. Pre-release validation
./agents/skills/release-management/scripts/validate-release.sh

# 3. Commit and push (GitHub Actions creates tag automatically)
git add -A && git commit -m "release: v0.2.0"
git push origin main
```

## Version Sync (Critical Step)

Before every release, run `./scripts/sync-version.sh <version>` to update:

| File | Update Type | Example |
|------|-------------|---------|
| Cargo.toml | Exact version | `0.2.0` |
| Cargo.lock | Regenerated | - |
| CHANGELOG.md | [Unreleased] → [0.2.0] with date | - |
| README.md | Major.minor compatibility | `0.2` |
| book/src/getting-started.md | Major.minor compatibility | `0.2` |
| wasm/package.json | Exact version | `0.2.0` |
| tests/*.rs | Exact version | `0.2.0` |
| examples/cli/*.sh | Exact version | `0.2.0` |
| llms.txt | Regenerated | - |

This prevents the common issue of stale versions in documentation.

### ⚠️ Important: Tags are Created Automatically

**DO NOT create git tags manually!** The release workflow automatically:
1. Extracts version from Cargo.toml
2. Creates and pushes the tag
3. Triggers the full release pipeline

### Script Distinction

| Script | Purpose | When to Use |
|--------|---------|-------------|
| `sync-version.sh <ver>` | Release automation | During release (used in CI) |
| `sync-docs.sh` | Documentation sync | Development, has `--check` mode |
| `check-docs-links.sh` | Validation + fix | CI validation, has `--fix` mode |

## Release Process Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Update version │────▶│  Push to main    │────▶│  CI Creates     │
│  in Cargo.toml  │     │  (git push)      │     │  Tag + Release  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

### How It Works
1. Update version in `Cargo.toml` and run `./scripts/sync-version.sh <version>`
2. Update `CHANGELOG.md` with release notes
3. Commit and push to main: `git push origin main`
4. GitHub Actions extracts version from Cargo.toml
5. CI creates tag `v*` automatically
6. CI builds, publishes to crates.io/npm, creates GitHub release

### Prerequisites
1. All conventional commits merged to main
2. `CHANGELOG.md` reflects changes with proper version header
3. CI passes on main branch
4. Trusted Publishing configured (see references/trusted-publishing.md)

## Validation Gates

Run `./scripts/pre-release-validate.sh` which checks:
- [ ] All README CLI commands work as documented
- [ ] All tests pass (`cargo test --all-features`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] LOC policy enforced (<= 500 lines per file)
- [ ] WASM build and size gate
- [ ] Version sync script available

For faster validation without benchmarks:
```bash
./scripts/pre-release-validate.sh --skip-bench
```

## CHANGELOG Requirements (CRITICAL)

The release workflow extracts changelog content using awk. **Incorrect formatting causes empty release notes.**

### Required Format
```markdown
## [0.2.9] - 2026-04-06

### Added
- Description of new features

### Changed
- Description of changes

### Fixed
- Description of fixes

[unreleased]: https://github.com/.../compare/v0.2.9...HEAD
[0.2.9]: https://github.com/.../releases/tag/v0.2.9
[0.2.8]: https://github.com/.../releases/tag/v0.2.8
```

### Common Mistakes (DO NOT DO)
```markdown
## [0.2.9]                    ❌ Duplicate/empty header
## [0.2.9] - 2026-04-06       ❌ Causes awk to exit immediately

## [0.2.9]                    ❌ Missing date
```

### Pre-Commit Validation
```bash
# Check for duplicate headers
grep -c '^\#\# \[.*\]' CHANGELOG.md | while read count; do
  [ "$count" -gt 1 ] && echo "❌ Duplicate header detected" && exit 1
done

# Check for version link entry at bottom
VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d'"' -f2)
grep -q "^\[${VERSION}\]:" CHANGELOG.md || echo "❌ Missing version link"
```

## Publishing Targets

| Target | Method | Trigger |
|--------|--------|---------|
| crates.io | Trusted Publishing (OIDC) | Push git tag `v*` |
| npm (if applicable) | `npm publish --provenance` | Push git tag `v*` |
| GitHub Release | `softprops/action-gh-release` | Push git tag `v*` triggers CI |
| GitHub Pages | mdBook + actions/deploy-pages | Push to main |

## CLI Usage Examples

### Full Release (Recommended)
```bash
# 1. Sync version across all files (prevents stale docs)
./scripts/sync-version.sh 0.2.0

# 2. Review changes
git diff

# 3. Run validation gates
cargo test --all-features
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# 4. Commit and push (GitHub Actions creates tag automatically)
git add -A && git commit -m "release: v0.2.0"
git push origin main

# 5. Monitor CI (GitHub Actions will create tag and release)
gh run watch

# 6. Verify publication
cargo search chaotic_semantic_memory
```

**⚠️ Important:** Do NOT create tags manually! GitHub Actions automatically creates tags from Cargo.toml version.

### Dry Run (Test Release Process)
```bash
# Test crates.io publishing without publishing
cargo publish --dry-run

# Test npm provenance (if applicable)
npm publish --dry-run --provenance
```

### Hotfix Release
```bash
# Create hotfix branch from tag
git checkout -b hotfix/v1.2.1 v1.2.0

# Apply fix and commit
git commit -m "fix: critical bug in reservoir spectral radius"

# Tag and push (triggers release workflow)
git tag -a v1.2.1 -m "Hotfix 1.2.1"
git push origin v1.2.1
```

### Rollback Failed Release
```bash
# Delete remote tag (before CI publishes)
git push --delete origin v1.2.0

# Yank from crates.io (if already published)
cargo yank --version 1.2.0 chaotic_semantic_memory

# Delete GitHub release
gh release delete v1.2.0 --yes
```

## Version Numbering

Follows [SemVer](https://semver.org/):
- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

Derived automatically from conventional commits:
- `feat!:` or `BREAKING CHANGE:` → MAJOR
- `feat:` → MINOR
- `fix:`, `perf:` → PATCH

## Detailed References

| Document | Purpose |
|----------|---------|
| [release-workflow.md](references/release-workflow.md) | Full workflow with CI examples |
| [trusted-publishing.md](references/trusted-publishing.md) | crates.io + npm OIDC setup |
| [version-tag-format.md](references/version-tag-format.md) | v{version} best practices, rolling tags |
| ADR-0049 | Release checklist and version sync protocol |

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "crate already exists" on crates.io | The workflow now checks and skips publish if already published |
| GitHub release not created but crates.io exists | The workflow now checks for existing releases and skips creation |
| Tag exists but release missing | Delete local/remote tag: `git tag -d vX.X.X && git push origin :refs/tags/vX.X.X`, then re-push |
| "OIDC token exchange failed" | Verify Trusted Publishing config on crates.io |
| "npm provenance failed" | Ensure Node 24+ and `id-token: write` permission |
| "npm token expired" | Generate fresh automation token at npmjs.com/settings/tokens |
| "npm 404 Not Found" | Package doesn't exist OR Trusted Publisher not configured |
| "Access token expired" | NPM_TOKEN secret is revoked; regenerate at npmjs.com |
| Docs not deploying | Check GitHub Pages settings → Source: GitHub Actions |

## Idempotent Releases (2026-03-17)

The release workflow is now idempotent - it handles partial failures gracefully:

1. **crates.io check**: Before publishing, checks if version already exists on crates.io
2. **GitHub release check**: Before creating release, checks if release already exists
3. **Skipped gracefully**: If either already exists, it skips that step and continues

This prevents the issue where:
- Crate was manually published to crates.io
- CI failed trying to re-publish
- GitHub release was never created
- Future runs skipped because tag existed

### Manual Recovery (if needed)

If a release partially failed:

```bash
# Check current state
gh release list
curl -s https://crates.io/api/v1/crates/chaotic_semantic_memory/versions | jq '.versions[0].num'
npm view @d-o-hub/chaotic_semantic_memory version

# If GitHub release missing but crates.io published:
# 1. Delete the tag locally and remotely
git tag -d vX.X.X && git push origin :refs/tags/vX.X.X

# 2. Re-create at correct commit (or HEAD)
git tag vX.X.X <commit> && git push origin vX.X.X
# OR create at HEAD:
git tag vX.X.X && git push origin vX.X.X

# 3. CI will now create the release (or manually with):
gh release create vX.X.X --title "vX.X.X" --notes-file CHANGELOG.md
```

## Security Requirements

- **Never** commit API tokens or secrets
- Use Trusted Publishing (OIDC) instead of long-lived tokens
- Require 2FA on crates.io and npm accounts
- Enable branch protection on main
