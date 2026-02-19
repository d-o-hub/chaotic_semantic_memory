---
name: release-management
description: GitHub release management, crates.io trusted publishing, npm provenance, and GitHub Pages documentation. Use when creating releases, publishing packages, or deploying docs.
---

# Release Management

Automated release pipeline using 2026 best practices: semantic-release for versioning, Trusted Publishing for crates.io/npm, and mdBook for docs.

## Quick Start

```bash
# Pre-release validation
.agents/skills/release-management/scripts/validate-release.sh

# Create GitHub release (after validation passes)
.agents/skills/release-management/scripts/create-github-release.sh v1.2.0
```

## Release Process Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Validate       │────▶│  Create Release  │────▶│  CI Publishes   │
│  (local checks) │     │  (tag + notes)   │     │  (auto-publish) │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

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
| crates.io | Trusted Publishing (OIDC) | Tag push `v*` |
| npm (if applicable) | `npm publish --provenance` | Tag push `v*` |
| GitHub Release | `gh release create` | Manual or CI |
| GitHub Pages | mdBook + actions/deploy-pages | Push to main |

## CLI Usage Examples

### Full Release (Recommended)
```bash
# 1. Validate everything
./scripts/validate-release.sh

# 2. Create and push tag
./scripts/create-github-release.sh v1.2.0

# 3. Monitor CI
gh run watch

# 4. Verify publication
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

# Tag and push
./scripts/create-github-release.sh v1.2.1
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

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "crate already exists" | Version bump required; check Cargo.toml |
| "OIDC token exchange failed" | Verify Trusted Publishing config on crates.io |
| "npm provenance failed" | Ensure Node 24+ and `id-token: write` permission |
| Docs not deploying | Check GitHub Pages settings → Source: GitHub Actions |

## Security Requirements

- **Never** commit API tokens or secrets
- Use Trusted Publishing (OIDC) instead of long-lived tokens
- Require 2FA on crates.io and npm accounts
- Enable branch protection on main
