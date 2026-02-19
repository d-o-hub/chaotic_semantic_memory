# Release Engineering

## Automated Releases

Releases are automated via GitHub Actions using:

- **Trusted Publishing** - OIDC-based authentication (no API tokens)
- **semantic-release** - Version bumping from conventional commits
- **npm provenance** - Supply chain verification

## Creating a Release

```bash
# 1. Validate release readiness
.agents/skills/release-management/scripts/validate-release.sh

# 2. Create and push tag
git tag v0.2.0
git push origin v0.2.0

# 3. GitHub Actions handles:
#    - crates.io publishing (Trusted Publishing)
#    - GitHub Release creation
#    - npm package publishing (WASM)
```

## Commit Conventions

| Type | Version Bump | Example |
|------|--------------|---------|
| `feat` | Minor | `feat(cli): add export command` |
| `fix` | Patch | `fix(reservoir): correct spectral radius` |
| `perf` | Patch | `perf(hyperdim): optimize bundle` |
| `BREAKING CHANGE` | Major | `feat!: redesign API` |

## Publishing Targets

| Target | Method | Trigger |
|--------|--------|---------|
| crates.io | Trusted Publishing (OIDC) | Tag push |
| npm | Provenance | Tag push |
| GitHub Release | gh CLI | Tag push |
| GitHub Pages | mdBook | Push to main |

## Documentation

Documentation is auto-deployed:

- **docs.rs** - Rust API docs (automatic from crates.io)
- **GitHub Pages** - mdBook guide (automatic from main branch)

## Manual Steps

1. Update CHANGELOG.md before release
2. Ensure Cargo.toml version matches tag
3. Run validation script locally
4. Review GitHub Actions workflow results

## Rollback

If a release has issues:

```bash
# Yank from crates.io
cargo yank --vers 0.2.0 chaotic_semantic_memory

# Delete GitHub release
gh release delete v0.2.0

# Delete tag
git push --delete origin v0.2.0
```

## Security

- No long-lived API tokens in CI
- Trusted Publishing Only mode enabled
- 2FA required for crate ownership
- Branch protection on main
