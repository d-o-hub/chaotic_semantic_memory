# Release Workflow (canonical)

Matches `.github/workflows/release.yml` as of Wave 32. Prefer the skill entrypoint `SKILL.md` for operator steps; this file holds detail and recovery.

## Trigger and tag owner

| Item | Value |
|------|-------|
| Workflow | `.github/workflows/release.yml` |
| Triggers | `push` to `main`, `workflow_dispatch` |
| Tag owner | Workflow job `validate`, step **Ensure release tag exists** |
| Tag format | `v` + version from root `Cargo.toml` |

```
merge PR to main
  → release.yml starts
  → wait-for-ci (gh run list --workflow=ci.yml --commit $SHA)
  → validate: changelog, sync-version clean, package list
  → git tag v$VERSION && git push origin v$VERSION   # ONLY tag owner
  → build matrix / publish crates / npm / GitHub release
```

**Humans do not create routine `v*` tags.** Manual tag push is recovery-only (below).

## Protected-main policy

1. Create `release/vX.Y.Z` (or `chore/release-…`) from updated `main`
2. Bump `Cargo.toml`, run `scripts/sync-version.sh`, fix CHANGELOG
3. Open PR; required CI must pass
4. Merge (squash preferred). Never `git push origin main` for releases
5. Watch `release.yml`; do not re-tag if it is still running

AGENTS.md / hard constraints: main is protected; multi-PR never use `gh pr merge --auto` (stale-base loops).

## CHANGELOG extraction pitfalls

`release.yml` requires:

- Exactly one `## [VERSION]` header
- Header includes date: `## [VERSION] - YYYY-MM-DD`
- Keep a Changelog sections as needed

`body_path` and `generate_release_notes: true` are mutually exclusive in `softprops/action-gh-release` — workflow uses changelog extraction, not dual modes.

## Idempotent behavior

1. If tag `vVERSION` already exists → `release-needed=false`, skip new tag
2. crates.io / npm paths should skip or tolerate already-published versions
3. GitHub release creation should not fail the whole world if re-run after partial success — verify with `gh release list`

## Recovery (approval-gated)

Only after confirming publish state and team approval:

```bash
# Inspect
gh release list --limit 5
gh run list --workflow=release.yml --limit 5
curl -s https://crates.io/api/v1/crates/chaotic_semantic_memory/versions | head

# Tag exists but release/publish incomplete — coordinate before deleting tags
git fetch --tags
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
# Re-merge or workflow_dispatch only with a clear plan; prefer fixing forward

# Already on crates.io and need yank
cargo yank --version X.Y.Z chaotic_semantic_memory
```

Prefer **fix-forward** (patch release) over deleting published artifacts.

## Local helpers

| Path | Role |
|------|------|
| `scripts/pre-release-validate.sh` | Canonical local gates |
| `scripts/sync-version.sh` | Propagate version |
| `scripts/release-manager.sh` | validate / prepare helpers |
| `.agents/skills/release-management/scripts/validate-release.sh` | Skill-local preflight |
| `.agents/skills/release-management/scripts/create-github-release.sh` | Manual GH release notes helper |

## Not used (stale docs)

This repository does **not** use a separate `semantic-release.yml` or npm `semantic-release` bot for tagging. Ignore older diagrams that show “push tag → release” as the human step; humans merge version PRs, workflow owns tags.

## Post-release checklist

- [ ] crates.io shows new version
- [ ] npm packages (WASM + CLI) updated if in scope
- [ ] GitHub Release notes non-empty
- [ ] Pages/docs still building
- [ ] No accidental second tag for same version
