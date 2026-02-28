# ADR-0049: Release Checklist and Version Sync Protocol

**Date:** 2026-02-27  
**Status:** Accepted  
**Author:** Release Engineering  

## Context

The v0.1.1 release exposed several issues:
1. crates.io token missing `publish-update` scope (403 Forbidden)
2. GitHub Actions artifact version mismatch (v6 vs v7)
3. Multiple version references across codebase not updated
4. No automated version sync across all files

This ADR establishes a comprehensive release checklist and version sync protocol to prevent recurrence.

## Decision

### 1. Release Checklist (Mandatory)

Before pushing any version tag, verify:

#### Pre-Release Validation
- [ ] All tests passing (`cargo test --all-features`)
- [ ] Format check passing (`cargo fmt --check`)
- [ ] Clippy passing (`cargo clippy --all-targets --all-features -- -D warnings`)
- [ ] LOC limits enforced (all src/*.rs files <= 500 LOC)
- [ ] Benchmarks passing (`cargo bench --bench benchmark`)
- [ ] No security vulnerabilities (`cargo audit` or dependabot resolved)

#### Version Sync Checklist
- [ ] `Cargo.toml` version updated
- [ ] `Cargo.lock` regenerated (`cargo update`)
- [ ] `CHANGELOG.md` - [Unreleased] section moved to [version] with date
- [ ] `README.md` version badge updated
- [ ] `book/src/getting-started.md` version updated
- [ ] `wasm/package.json` version updated (if WASM changes)
- [ ] All test files with hardcoded version strings updated
- [ ] All example scripts with version references updated
- [ ] `llms.txt` and `llms-full.txt` regenerated

#### crates.io Token Verification
- [ ] Token has `publish-new` scope OR
- [ ] Token has `publish-update` scope for existing crates (CRITICAL for v0.1+)
- [ ] Token scope verified at https://crates.io/settings/tokens
- [ ] GitHub secret `CARGO_REGISTRY_TOKEN` updated if token regenerated

#### npm Token Verification (if WASM changes)
- [ ] Token has appropriate npm publishing permissions OR
- [ ] OIDC trusted publisher configured
- [ ] npm >= 11.5.1 available in CI

#### Git Workflow
- [ ] Version tag created (`git tag -a v<x>.<y>.<z> -m "Release version <x>.<y>.<z>"`)
- [ ] Tag pushed (`git push origin v<x>.<y>.<z>`)
- [ ] Release workflow completes successfully
- [ ] GitHub Release created automatically

#### GitHub Release Verification
- [ ] Release name uses `v{version}` format (e.g., `v0.1.3`), NOT `{package} v{version}`
- [ ] Release body contains full changelog, not just auto-generated compare link
- [ ] Verify with `gh release list` and `gh release view "v{x}.{y}.{z}"`

#### Post-Release Verification
- [ ] crates.io shows new version published
- [ ] docs.rs documentation updated
- [ ] GitHub Release assets present
- [ ] npm package updated (if applicable)

### 2. Version Sync Script

Use `scripts/sync-version.sh` to automate version updates:

```bash
# Preview changes
./scripts/sync-version.sh 0.2.0 --dry-run

# Apply version update
./scripts/sync-version.sh 0.2.0
```

The script updates:
- Cargo.toml (exact version)
- Cargo.lock
- CHANGELOG.md ([Unreleased] → [version])
- README.md
- book/src/getting-started.md
- wasm/package.json
- tests/*.rs
- examples/cli/*.sh
- Regenerates llms.txt

### 3. Version Reference Locations

Maintain a complete list of files that may contain version references:

| File | Version Format | Auto-syncable |
|------|---------------|---------------|
| Cargo.toml | `"0.1.3"` | Yes |
| Cargo.lock | `"0.1.3"` | Yes (cargo update) |
| CHANGELOG.md | `[0.1.3]` | Yes |
| README.md | `0.1.3` | Yes |
| book/src/*.md | `0.1.3` | Yes |
| wasm/package.json | `"0.1.3"` | Yes |
| tests/*.rs | `"0.1.3"` | Yes |
| examples/cli/*.sh | `0.1.3` | Yes |
| plans/adr/*.md | `v0.1.3` | Manual |
| progress/LEARNINGS.md | v0.1.3 | Manual |
| llms.txt | `0.1.3` | Yes (gen-llms-txt.sh) |

### 4. Token Scope Reference

| Registry | New Crate Scope | Existing Crate Scope |
|----------|-----------------|---------------------|
| crates.io | `publish-new` | `publish-update` (CRITICAL) |
| npm | `publish` | `publish` |

## Consequences

### Positive
- Eliminates version sync oversights
- Prevents 403 errors from incorrect token scopes
- Standardizes release process across team
- Enables automated release verification

### Negative
- Additional pre-release checklist to follow
- Requires manual token scope verification (cannot be automated)

## Alternatives Considered

1. **Fully automated release script** - Rejected because token scopes require manual verification at provider UI
2. **Separate release branch** - Rejected; tag-based workflow is simpler
3. **Release-please automation** - Deferred; current workflow is sufficient for now

## References

- Learnings: `progress/LEARNINGS.md` - v0.1.3 Release section
- Release workflow: `.github/workflows/release.yml`
- npm workflow: `.github/workflows/npm-publish.yml`
- npm workflow: `.github/workflows/npm-publish.yml`
