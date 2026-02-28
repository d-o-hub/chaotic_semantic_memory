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

# 3. Create git tag (push triggers release.yml workflow)
git tag -a v0.2.0 -m "Release 0.2.0"
git push origin v0.2.0
```

## Version Sync (Critical Step)

Before every release, run `./scripts/sync-version.sh <version>` to update:

| File | Update Type | Example |
|------|-------------|---------|
| Cargo.toml | Exact version | `0.2.0` |
| Cargo.lock | Regenerated | - |
| CHANGELOG.md | [Unreleased] → [0.2.0] | - |
| README.md | Major.minor compatibility | `0.2` |
| book/src/getting-started.md | Major.minor compatibility | `0.2` |
| wasm/package.json | Exact version | `0.2.0` |
| tests/*.rs | Exact version | `0.2.0` |
| examples/cli/*.sh | Exact version | `0.2.0` |
| llms.txt | Regenerated | - |

This prevents the common issue of stale versions in documentation.

## Release Process Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Validate       │────▶│  Push Git Tag    │────▶│  CI Creates     │
│  (local checks) │     │  (git push v*)   │     │  GitHub Release │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

### How It Works
1. User creates git tag locally: `git tag -a v1.2.0 -m "Release 1.2.0"`
2. User pushes tag: `git push origin v1.2.0`
3. GitHub Actions triggers on `v*` tag push
4. CI builds, publishes to crates.io, creates GitHub release with artifacts

### Prerequisites
1. All conventional commits merged to main
2. `CHANGELOG.md` reflects changes (auto-generated via semantic-release)
3. CI passes on main branch
4. Trusted Publishing configured (see references/trusted-publishing.md)

## Validation Gates

Run `scripts/validate-release.sh` which checks:
- [ ] All tests pass (`cargo test --all-features`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] Version in Cargo.toml matches planned release
- [ ] No uncommitted changes in working tree

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

# 4. Commit and tag
git add -A && git commit -m "release: v0.2.0"
git tag -a v0.2.0 -m "Release 0.2.0"
git push origin main v0.2.0

# 5. Monitor CI
gh run watch

# 6. Verify publication
cargo search chaotic_semantic_memory
```

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
| "crate already exists" | Version bump required; check Cargo.toml |
| "OIDC token exchange failed" | Verify Trusted Publishing config on crates.io |
| "npm provenance failed" | Ensure Node 24+ and `id-token: write` permission |
| "npm token expired" | Generate fresh automation token at npmjs.com/settings/tokens |
| "npm 404 Not Found" | Package doesn't exist OR Trusted Publisher not configured |
| "Access token expired" | NPM_TOKEN secret is revoked; regenerate at npmjs.com |
| Docs not deploying | Check GitHub Pages settings → Source: GitHub Actions |

## Security Requirements

- **Never** commit API tokens or secrets
- Use Trusted Publishing (OIDC) instead of long-lived tokens
- Require 2FA on crates.io and npm accounts
- Enable branch protection on main
