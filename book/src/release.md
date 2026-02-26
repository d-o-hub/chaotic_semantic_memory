# Release Engineering

## Overview

Releases are automated via `scripts/release-manager.sh` with GitHub Actions for publishing. The system uses:

- **Structured logging** with JSON output for CI debugging
- **Exit-code validation** (no fragile grep patterns)
- **Rollback automation** for failed releases
- **Version sync** across Cargo.toml and wasm/package.json

## Release Manager

```bash
# Validate all gates (tests, clippy, fmt, LOC, dry-run)
scripts/release-manager.sh validate

# Prepare release (version bump, changelog, sync)
scripts/release-manager.sh prepare 0.2.0

# Publish (creates git tag, pushes - triggers GitHub release via CI)
scripts/release-manager.sh publish 0.2.0

# Full pipeline (validate + prepare + publish)
scripts/release-manager.sh full 0.2.0

# CI mode (non-interactive)
scripts/release-manager.sh full 0.2.0 --yes --log release.log

# Dry run (simulate without side effects)
scripts/release-manager.sh full 0.2.0 --dry-run
```

## Validation Gates

The `validate` command checks:

| Gate | Method |
|------|--------|
| Clean workspace | `git diff --quiet` |
| Correct branch | Must be `main` |
| Compilation | `cargo check --all-targets --all-features` |
| Formatting | `cargo fmt --check` |
| Linting | `cargo clippy -- -D warnings` |
| Tests | `cargo test --all-features` |
| Documentation | `cargo doc --no-deps` |
| Publish dry-run | `cargo publish --dry-run` |
| LOC limits | All src/*.rs ≤ 500 lines |
| WASM target | `cargo check --target wasm32-unknown-unknown` |
| Security audit | `cargo audit` (if installed) |

## Documentation Sync

The `prepare` command automatically updates version references across all documentation:

| File | Updates |
|------|---------|
| `Cargo.toml` | `version = "X.Y.Z"` |
| `wasm/package.json` | `"version": "X.Y.Z"` |
| `Cargo.lock` | Regenerated via `cargo check` |
| `README.md` | Status table + install examples |
| `SECURITY.md` | Supported versions table |
| `book/src/getting-started.md` | Install examples |
| `wasm/README.md` | npm install examples |
| `llms.txt`, `llms-full.txt` | Regenerated via `scripts/gen-llms-txt.sh` |
| `CHANGELOG.md` | [Unreleased] → [X.Y.Z] |
| `AGENTS.md` | Version references (if present) |

## CI Workflows

### Release (`release.yml`)

Triggered by git tag push (`v*`):

1. **validate** — Extract version from git tag, match Cargo.toml, dry-run publish
2. **build-artifacts** — Build release binary + WASM, create tarballs
3. **publish-crates** — `cargo publish` to crates.io
4. **create-github-release** — Upload artifacts, extract changelog notes
5. **notify** — Report success or failure with per-job status table
6. **update-rolling-tags** — Update major/minor tags (v1, v1.2)

### npm Publish (`npm-publish.yml`)

Triggered by tag push (`v*`):
- Builds WASM package via `wasm-pack`
- Publishes `@d-o-hub/chaotic-semantic-memory` to npm
- Includes npm provenance (`--provenance`)

### GitHub Pages (`pages.yml`)

Triggered by push to `main` (book/ changes):
- Builds mdBook documentation
- Generates API docs via `cargo doc`
- Deploys to GitHub Pages

## Commit Conventions

| Type | Version Bump | Example |
|------|--------------|---------|
| `feat` | Minor | `feat(cli): add export command` |
| `fix` | Patch | `fix(reservoir): correct spectral radius` |
| `perf` | Patch | `perf(hyperdim): optimize bundle` |
| `feat!:` | Major | `feat!: redesign API` |
| `docs`, `chore`, `test`, `ci` | No release | — |

## Rollback

If a release has issues:

```bash
# Automated rollback (deletes tag + GitHub release)
scripts/release-manager.sh rollback 0.2.0

# If already published to crates.io, yank manually:
cargo yank --version 0.2.0 chaotic_semantic_memory
```

## Security

- **No long-lived API tokens** — Uses CARGO_REGISTRY_TOKEN secret scoped to environment
- **Concurrency control** — Release workflow uses `cancel-in-progress: false`
- **Minimal permissions** — Only `contents: write` + `id-token: write`
- **Branch protection** — `main` branch requires PR with passing CI
- **Provenance** — npm packages include build provenance attestation
- **Audit** — `cargo audit` runs as part of validation (when installed)

## Architecture Decision Records

- [ADR-0039](../plans/adr/0039-release-engineering.md) — Release engineering strategy
- [ADR-0042](../plans/adr/0042-release-automation-v010.md) — Release automation and v0.1.0 readiness
