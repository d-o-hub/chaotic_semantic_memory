# ADR-0042: Release Automation and v0.1.0 Readiness

## Status

Proposed

## Context and Problem Statement

Comprehensive analysis of the release infrastructure reveals 13 issues blocking a clean v0.1.0 release to crates.io and GitHub:

### Critical Blockers

1. **Broken CI action**: `release.yml` line 102 uses `rust-lang/crates-io-auth-action@v1` which does not exist as a published GitHub Action. The workflow will fail on any tag push.
2. **No version tag**: `git tag -l` is empty despite CHANGELOG.md listing `[0.1.0]` dated 2026-02-17.
3. **Dirty workspace**: 20+ untracked files (`1`, `examples/cli/*.sh`, `examples/tmp/`, `.opencode/`) cause `cargo publish --dry-run` to fail.
4. **CHANGELOG mismatch**: `[Unreleased]` section has post-0.1.0 changes that should be merged into the release.

### Script Reliability

5. **Interactive prompts**: Both `validate-release.sh` and `create-github-release.sh` use `read -p` which blocks CI/automation usage.
6. **Fragile validation**: `validate-release.sh` uses `grep -q "^error"` on cargo output instead of checking exit codes.
7. **Double test runs**: `validate-release.sh` runs `cargo test` twice.
8. **No dry-run**: No `cargo publish --dry-run` step anywhere in the validation pipeline.

### Workflow Issues

9. **npm token mismatch**: `npm-publish.yml` uses `secrets.NPM_TOKEN` while ADR-0039 claims OIDC Trusted Publishing.
10. **Notify always succeeds**: Release notify job reports success regardless of actual publish-crates outcome.
11. **Duplicate release creation**: `create-github-release.sh` and `release.yml` both create GitHub Releases.
12. **No rollback**: No automation for rolling back a partial release failure.

### Documentation Sync (Automated)

13. **CONTRIBUTING.md stale**: References MSRV 1.82, missing pre-commit hooks, no release process.
14. **ADR-0039 still "Proposed"**: Should be "Accepted" since Wave 11 implemented the infrastructure.
15. **Book release.md inaccurate**: References non-functional scripts and incorrect process flow.
16. **Version sync incomplete**: README.md, SECURITY.md, book/src/getting-started.md, wasm/README.md, llms.txt not auto-updated on release.

## Decision Drivers

- `cargo publish --dry-run` must pass clean before any release
- All scripts must be non-interactive (CI-compatible) with `--yes` flag for automation
- Release workflow must use valid, existing GitHub Actions
- Single source of truth for version (Cargo.toml) synced to package.json
- Proper error handling with structured logging and rollback support
- 2026 GitHub security best practices (Trusted Publishing, provenance, branch protection)

## Considered Options

### Option 1: Patch existing scripts
Fix individual issues in validate-release.sh and create-github-release.sh.

### Option 2: Unified release-manager.sh with full error handling (Chosen)
Create a single orchestration script that replaces both scripts with proper logging, error handling, rollback, and CI compatibility.

### Option 3: cargo-release
Use `cargo-release` crate for automation.

## Decision Outcome

**Option 2**: Unified `scripts/release-manager.sh` because it provides:
- Single entry point for all release operations
- Structured logging with timestamps
- Rollback on partial failure
- Non-interactive mode for CI, interactive mode for humans
- Version sync across Cargo.toml and wasm/package.json
- CHANGELOG automation ([Unreleased] → [version])
- `cargo publish --dry-run` validation gate
- **Documentation sync**: Auto-updates README.md, SECURITY.md, book/, wasm/README.md, llms.txt

### Automated Documentation Sync

The `prepare` command automatically updates version references in:

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

### Implementation Plan (9 Actions)

| # | Action | Cost | Dependencies |
|---|--------|------|-------------|
| 1 | Clean workspace cruft, update .gitignore | 1 | None |
| 2 | Fix release.yml: replace broken action with `cargo publish` | 2 | None |
| 3 | Fix release.yml notify job error handling | 1 | Action 2 |
| 4 | Create `scripts/release-manager.sh` with logging + error handling | 4 | None |
| 5 | Add doc sync to release-manager.sh (README, SECURITY, book, wasm, llms.txt) | 2 | Action 4 |
| 6 | Update CONTRIBUTING.md (MSRV, hooks, release process) | 1 | None |
| 7 | Update book/src/release.md with accurate process | 1 | Action 4 |
| 8 | Merge CHANGELOG [Unreleased] into [0.1.0], update ADR-0039 status | 1 | None |
| 9 | Update ACTIONS.md with Phase 26 actions | 1 | None |

### Positive Consequences

- Reproducible, automated releases from a single script
- `cargo publish --dry-run` catches packaging errors before tag push
- Structured log output enables CI debugging
- Rollback automation prevents half-published states
- Clean workspace passes all cargo packaging checks
- Documentation accurately reflects actual release process
- **All user-facing docs auto-sync on release**: README.md, SECURITY.md, book/, wasm/README.md, llms.txt

### Negative Consequences

- Single script is a larger maintenance surface than separate small scripts
- Teams must learn new `release-manager.sh` interface
- Rollback cannot undo crates.io publish (only yank)

## Security (2026 Best Practices)

- **Trusted Publishing**: OIDC tokens, no long-lived secrets in CI
- **Permissions**: Minimal `contents: write` + `id-token: write` only
- **Provenance**: npm `--provenance` for WASM package
- **Branch protection**: `main` branch protected, PRs required
- **Dependency audit**: `cargo audit` in validation pipeline
- **Artifact signing**: GitHub attestation on release artifacts
- **Concurrency control**: Cancel-in-progress prevents race conditions

---

**Created:** 2026-02-20
**Author:** Release Engineering Analysis
**Supersedes:** Portions of ADR-0039 (implementation details)
