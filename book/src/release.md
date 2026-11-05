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

Triggered by successful completion of the `CI` workflow on `main` (and by a
fail-closed manual dispatch). It uses the completed workflow's `head_sha`, not
the SHA of a later workflow run:

1. **validate** — Extract version from Cargo.toml, verify CI + gate for the target SHA, ensure the tag exists, validate package assembly
2. **build-artifacts** — Build release binary + WASM, create tarballs
3. **create-github-release** — Upload artifacts, extract changelog notes
4. **trigger-crates-io-publish** — Dispatch `publish-crates.yml` for the tag just created and wait for its verdict
5. **verify-release** — Probe all nine crates.io index paths, both npm packages and every expected release asset
6. **notify** — Report the release verdict with per-job status

`release.yml` holds **no crates.io credential and no `cargo publish`**. crates.io refuses
OIDC tokens minted from the `workflow_run` event that triggers this workflow, so the
publishing itself lives in `publish-crates.yml` (below) and this file only causes that
workflow to run and reads its outcome. `verify-release` still probes the nine index paths
independently, so a release whose publish failed, timed out, or never started stays red
even if the dispatch job were to be removed again.

### npm distribution

The npm package is published by the npm distribution job owned by the release
workflow; there is no separate `npm-publish.yml` workflow:
- Builds the WASM package via `scripts/build-wasm.sh release-web` (the same script and mode CI smoke-tests)
- Publishes `@d-o-hub/chaotic_semantic_memory` to npm
- Includes npm provenance (`--provenance`)

### crates.io (`publish-crates.yml`)

`workflow_dispatch` only — inputs `ref` (e.g. `refs/tags/v0.3.9`), `version`, `dry_run`.
It mints a short-lived token through crates.io OIDC Trusted Publishing
(`rust-lang/crates-io-auth-action`, no repository secret), confirms every crate in the
workspace declares the dispatched version, publishes the seven members, then
`chaotic_semantic_memory`, then `csm-duckdb`, and finally re-probes the sparse index for
all nine and reds the run on any gap. Publishing is idempotent: a crate already in the
index at that version is skipped, so re-dispatching finishes a partial run.

A release run dispatches it automatically (`trigger-crates-io-publish`), which posts the
dispatch, finds the run it caused by the release commit's SHA, and waits for that run's
verdict — 38 minutes of waiting at most (3 to find the run, 35 for it to finish) inside
a 40-minute job. That job is red if the run never appears (the usual cause is
`publish-crates.yml` not being on the default branch yet, since `workflow_dispatch` only
fires for files that have merged), if it finishes anything but `success`, or if an
environment approval holds it past the bound.

The same workflow, dispatched by hand, is the recovery path:

```bash
gh workflow run publish-crates.yml -f ref=refs/tags/v0.3.9 -f version=0.3.9 -f dry_run=true
```

`--dry-run` packs without uploading and therefore never performs the OIDC exchange: it
proves packaging and version agreement, **not** that the trusted-publisher configs exist.
Omitting it publishes for real — which is why a release run omits it, and why the dry run
above is spelled out for a human. See ADR-0104 and `agents-docs/release-safety.md` for the
nine configs a release needs.

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

- **No long-lived API tokens** — crates.io publishes over OIDC Trusted Publishing from
  `publish-crates.yml`; no workflow reads a `CARGO_REGISTRY_TOKEN` secret (ADR-0104)
- **Concurrency control** — Release and publish workflows use `cancel-in-progress: false`
- **Minimal permissions** — `release.yml` needs `contents: write`; its crates.io dispatch job
  needs `contents: read` + `actions: write` and mints no token of its own;
  `publish-crates.yml` needs only `contents: read` + `id-token: write`, under a
  repository-wide `permissions: {}`
- **Branch protection** — `main` branch requires PR with passing CI
- **Provenance** — npm packages include build provenance attestation
- **Audit** — `cargo audit` runs as part of validation (when installed)

## Architecture Decision Records

- [ADR-0039](../../plans/adr/0039-release-engineering.md) — Release engineering strategy
- [ADR-0042](../../plans/adr/0042-release-automation-v010.md) — Release automation and v0.1.0 readiness
- [ADR-0104](../../plans/adr/0104-crates-io-oidc-trusted-publishing.md) — crates.io OIDC trusted publishing
