---
name: release-management
description: GitHub release management, crates.io trusted publishing, npm provenance, and GitHub Pages documentation. Use when creating releases, publishing packages, or deploying docs.
---

# Release Management

Automated release pipeline: version sync, Trusted Publishing (crates.io/npm), and docs. **Main is protected** — never push release commits directly to `main`.

## Hard rules

1. **Never push directly to `main`.** Use branch → PR → required CI green → merge.
2. **Never create release tags manually for routine releases.** Tag owner is `.github/workflows/release.yml` (job `validate` / step “Ensure release tag exists”).
3. **Single trigger:** workflow runs on **push to `main`** (after merge) or `workflow_dispatch`. It waits for CI, then creates `v{version}` from `Cargo.toml` and publishes.
4. Identify the artifact channel first via the `dist-channel-selection` skill.

## Protected release flow

```
feature branch → PR → CI green → merge to main
        → release.yml (wait-for-ci) → create tag v* → publish
```

### Operator steps

```bash
# 1. Branch from up-to-date main
git checkout main && git pull
git checkout -b release/v0.2.0

# 2. Set version in Cargo.toml, then sync dependents
#    (edit Cargo.toml version = "0.2.0")
./scripts/sync-version.sh 0.2.0

# 3. CHANGELOG: one dated header, Keep a Changelog sections + footer links
#    ## [0.2.0] - YYYY-MM-DD

# 4. Local gates
./scripts/pre-release-validate.sh --skip-bench
# Optional skill-local extras:
.agents/skills/release-management/scripts/validate-release.sh

# 5. Commit on the branch — never on main
git add -A
git commit -m "release: v0.2.0"

# 6. PR → wait for CI → merge (squash preferred; no --auto multi-PR loops)
git push -u origin HEAD
gh pr create --title "release: v0.2.0" --body "Version bump and changelog for 0.2.0"
gh pr checks --watch
gh pr merge --squash

# 7. After merge: release.yml owns the tag and publish
gh run list --workflow=release.yml --limit 3
gh run watch
```

**Do not** `git push origin main` or `git tag` / `git push origin v*` for normal releases. The workflow extracts version from `Cargo.toml`, creates `v${VERSION}`, and pushes the tag.

## Version sync (before PR)

Run `./scripts/sync-version.sh <version>` after editing `Cargo.toml`. It updates Cargo.lock, CHANGELOG date section, README/docs minor pins, `wasm/package.json`, tests/examples, and regenerates llms files as configured.

| Script | Purpose |
|--------|---------|
| `scripts/sync-version.sh <ver>` | Release version propagation |
| `scripts/sync-docs.sh` | Doc sync (`--check` in CI) |
| `scripts/check-docs-links.sh` | Links + version consistency |
| `scripts/pre-release-validate.sh` | Full pre-release gates |
| `scripts/release-manager.sh` | Unified validate/prepare helpers |

## CHANGELOG (CRITICAL)

Release note extraction requires **exactly one** header with a date:

```markdown
## [0.2.0] - 2026-04-06

### Added
- …

[0.2.0]: https://github.com/d-o-hub/chaotic_semantic_memory/releases/tag/v0.2.0
```

Avoid duplicate headers and missing dates — both fail `release.yml` validate.

```bash
VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d'"' -f2)
grep -c "^## \[${VERSION}\]" CHANGELOG.md   # must be 1
grep -q "^## \[${VERSION}\] - [0-9]" CHANGELOG.md
grep -q "^\[${VERSION}\]:" CHANGELOG.md
```

## Distribution channels

1. **Rust library:** `chaotic_semantic_memory` (crates.io, Trusted Publishing / OIDC)
2. **JS/WASM:** `@d-o-hub/chaotic_semantic_memory` (npm provenance)
3. **CLI:** `@d-o-hub/csm` (npm)

| Target | Method | When |
|--------|--------|------|
| crates.io | OIDC Trusted Publishing | After tag from release workflow |
| npm packages | `npm publish --provenance` / OIDC | Same |
| GitHub Release | `softprops/action-gh-release` | Same |
| GitHub Pages | mdBook deploy | Docs path on main / release jobs |

## crates.io name verification (FIRST PUBLISH)

Before first publish, verify workspace crate names are available on crates.io:

```bash
for crate in csm-chaos csm-core-lib csm-traits csm-embedding csm-memory csm-retrieval csm-persistence; do
  echo -n "$crate: "
  cargo search "$crate" 2>/dev/null | head -1
done
```

If a name is taken by an unrelated project (e.g., `csm-core-lib` is "Candle-based inference for Sesame CSM-1B"), rename the crate before publishing. Use prefixes like `chaotic-semantic-memory-*` or suffixes like `csm-core-lib-lib` to avoid conflicts.

**Known conflicts (as of 2026-07-22):**
- `csm-core-lib`: Taken by Sesame CSM-1B TTS project
- `csm-chaos`: Available (published at 0.3.7)
- `csm-traits`, `csm-embedding`, `csm-memory`, `csm-retrieval`, `csm-persistence`, `csm-wasm`: Not yet published, availability unknown

## GitHub environment configuration

The `crates.io` environment may have protection rules that block automatic publishing:

- **Wait timer**: Default 15 minutes. Remove in GitHub Settings → Environments → crates.io if not needed.
- **Required reviewers**: Must be configured if manual approval is needed.
- **Deployment branches**: Restrict to `main` only.

Check pending deployments before re-running failed releases:
```bash
gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments
```

## Tag ownership (single owner)

| Who | Action |
|-----|--------|
| **Human / PR** | Bump `Cargo.toml`, changelog, sync-version, merge via PR |
| **`release.yml`** | Wait for CI → create+push `v{version}` → package → publish → GitHub release |

If the tag already exists, the workflow sets `release-needed=false` and skips re-publish paths (idempotent). Manual tag push is **recovery-only**, approval-gated — see `references/release-workflow.md`.

## Dry run / hotfix / rollback

```bash
cargo publish --dry-run
# npm: npm publish --dry-run --provenance (package dir as applicable)
```

Hotfix: still use a branch + PR into `main`; after merge, `release.yml` creates the patch tag. Do not tag from a side branch for routine hotfixes.

Rollback / partial failure recovery (yank, delete tag, recreate) is documented in `references/release-workflow.md` and must be human-approved.

## Validation gates (local)

`./scripts/pre-release-validate.sh` checks tests, clippy, docs, LOC, WASM size, version sync, etc. Faster:

```bash
./scripts/pre-release-validate.sh --skip-bench
```

Also: `cargo deny check` before release when deps changed.

## SemVer (from conventional commits)

- **MAJOR:** `feat!:` / `BREAKING CHANGE:`
- **MINOR:** `feat:`
- **PATCH:** `fix:`, `perf:`

## References

| Document | Purpose |
|----------|---------|
| [release-workflow.md](references/release-workflow.md) | Full flow, recovery, pitfalls |
| [trusted-publishing.md](references/trusted-publishing.md) | crates.io + npm OIDC |
| [version-tag-format.md](references/version-tag-format.md) | `v{version}` conventions |
| ADR-0039 / ADR-0049 | Release engineering / checklist |

Skill scripts (optional, not the tag owner):

- `scripts/validate-release.sh` — skill-local preflight
- `scripts/create-github-release.sh` — manual GH release helper (recovery)

## Security

- Never commit API tokens; prefer OIDC Trusted Publishing
- 2FA on crates.io and npm
- Branch protection on `main` (required checks, no direct push)
