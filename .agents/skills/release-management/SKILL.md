---
name: release-management
description: GitHub release management, crates.io trusted publishing, npm provenance, and GitHub Pages documentation. Use when creating releases, publishing packages, or deploying docs.
---

# Release Management

Protected release pipeline for `chaotic_semantic_memory`:
branch → PR → CI green → merge to main → `release.yml` tags and publishes.
No step may be shortcut, and humans never create tags.

## Guardrails (must hold)

1. **Branch + PR**: every change ships through a PR to `main` (`ci.yml` runs on every PR).
2. **CI green before merge**: `ci.yml` calls `scripts/validate.sh`; it must pass before anything merges.
3. **Merge to `main` only when green**.
4. **Tag + release are automatic and single-owner**: on push to `main`, `release.yml` `wait-for-ci` blocks until that commit's `ci.yml` run is `success`, then its `validate` job creates and pushes `v<version>` — the only tag creator. `dist.yml` consumes existing `v*` tags (attaches cargo-dist binaries); it never creates tags.
5. **Never run `git tag` / `git push` tags manually**, and never run `release-manager.sh publish` to tag (duplicates the CI-owned tag). If the tag already exists the workflow is idempotent and skips re-publishing.
6. **Destructive recovery is human-only** — yank/rollback/tag deletion require explicit human approval (see below); never run them in CI.

## Release steps (one canonical pass)

```bash
# 1. Bump version and sync every file (Cargo.toml, wasm/package.json, README,
#    book, SECURITY.md, CHANGELOG.md, Cargo.lock, llms.txt)
scripts/release-manager.sh prepare <version>

# 2. Changelog entry — see format below.
# 3. Local pre-flight gates (includes cargo publish --dry-run)
scripts/release-manager.sh validate

# 4. Commit ANY version-sync changes (release.yml hard-fails on uncommitted
#    sync output), open the PR, verify CI is green, merge to main.
# 5. Monitor: gh run watch   -> then verify:
#    gh release view v<version> && cargo search chaotic_semantic_memory
```

After the merge, `release.yml` on `main` runs: wait-for-CI gate → changelog/version validation → tag `v<version>` → crates.io publish (workspace crates in dependency order, OIDC token + fallback) → CLI/WASM builds → GitHub Release with changelog notes → npm publishes `@d-o-hub/chaotic_semantic_memory` and `@d-o-hub/csm` (OIDC provenance + token fallback) → `verify-release` checks every registry.

## Validation gates

- CI/PR gate: `scripts/validate.sh` — `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, source/crates LOC ≤ 500/file, WASM build + size gate, `scripts/gen-llms-txt.sh` (api-surface ≤ 5000 symbols), ADR registry check.
- Operator pre-flight: `scripts/release-manager.sh validate` — same gates plus `cargo publish --dry-run` and tag-availability check.

## CHANGELOG format (release notes are extracted with awk)

```markdown
## [0.2.9] - 2026-04-06

### Added
- …

[unreleased]: https://github.com/d-o-hub/chaotic_semantic_memory/compare/v0.2.9...HEAD
[0.2.9]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.9
```

- Header MUST be exactly `## [<version>] - YYYY-MM-DD`, unique (duplicate headers abort), with the `[<version>]:` compare link at the bottom.
- Version numbering follows SemVer; derive MAJOR from `feat!`/`BREAKING CHANGE`, MINOR from `feat:`, PATCH from `fix:`/`perf:`. See `references/version-tag-format.md`.

## Distribution channels

This repo publishes three artifacts — consult the `dist-channel-selection` skill first:
1. Crate `chaotic_semantic_memory` (crates.io, Trusted Publisher).
2. `@d-o-hub/chaotic_semantic_memory` (npm, OIDC + fallback).
3. `@d-o-hub/csm` (npm, OIDC + fallback).

Trusted-Publishing wiring: `references/trusted-publishing.md` (ADR: `plans/adr/0046-npm-oidc-trusted-publishing.md`).
docs/changelog/release automation: `references/release-workflow.md`, ADR `plans/adr/0042-release-automation-v010.md`.

## Rollback / yank — requires explicit human approval

`rollback` deletes the tag and GitHub release; `cargo yank` removes a published crate version from crates.io. Both are irreversible and MUST NOT be automated or run with `--yes` (it skips the interactive confirmation). Run interactively only, after a human dictates the exact version:

```bash
scripts/release-manager.sh rollback <version>          # prompts before deleting tag/release
cargo yank --version <version> chaotic_semantic_memory # human confirms registry state first
```

## Operational notes

- Idempotent releases: `release.yml` skips registry steps whose version already exists.
- Tag exists but release missing: with human approval delete tag + release (`scripts/release-manager.sh rollback <version>`, or the manual one-liner in its header), then push a fresh commit to main so release.yml re-runs.
- GitHub Pages docs: mdBook workspace under `book/` deployed by `pages.yml` on push to `main` (set Pages source to "GitHub Actions").
- Tokens/secrets: never commit them; prefer OIDC/Trusted Publishing over long-lived tokens; require 2FA on crates.io/npm accounts.

## References

- `.gitignore`/`pkg`: `scripts/release-manager.sh`, `scripts/validate.sh`, `scripts/sync-version.sh`, `scripts/gen-llms-txt.sh`
- Workflows: `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `.github/workflows/dist.yml`, `.github/workflows/pages.yml`
- Skill docs: `references/release-workflow.md`, `references/trusted-publishing.md`, `references/version-tag-format.md`