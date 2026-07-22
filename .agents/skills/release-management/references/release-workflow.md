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

## Recovery dispatch (workflow_dispatch)

For partially published releases (e.g., npm succeeded but crates.io failed), use recovery dispatch:

```bash
# Retry missing artifacts for existing tag
gh workflow run release.yml --ref main -f recover=true
```

The recovery workflow:
1. Sees tag exists, but `recover=true` → proceeds with `release-needed=true`
2. Each job checks if its artifact already exists → skips if so
3. Publishes only missing artifacts (idempotent)

**Prerequisites for recovery:**
- `CARGO_REGISTRY_TOKEN` must belong to an owner of ALL workspace crates on crates.io
- crates.io environment must not have a wait timer blocking the `publish-crates` job
- Workspace crate names must not be taken by unrelated projects (see below)

## crates.io name conflicts

Workspace crate names (`csm-core`, `csm-traits`, etc.) may be taken by unrelated projects on crates.io. If so, `cargo publish` will fail with:

```
403 Forbidden: this crate exists but you don't seem to be an owner.
```

**Solution:** Rename crates before first publish. Use prefixes like `chaotic-semantic-memory-*` or suffixes like `csm-core-lib`.

```bash
# Check if names are available
for crate in csm-chaos csm-core csm-traits csm-embedding csm-memory csm-retrieval csm-persistence; do
  echo -n "$crate: "
  cargo search "$crate" 2>/dev/null | head -1
done
```

**Known conflicts (as of 2026-07-22):**
- `csm-core`: Taken by Sesame CSM-1B TTS project (v0.1.0)

## Environment wait timer

The `crates.io` GitHub environment may have a wait timer (default 15 minutes). This blocks the `publish-crates` job until the timer expires.

```bash
# Check pending deployments
gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments
# Look for wait_timer > 0
```

**Solution:** Remove the timer in GitHub Settings → Environments → crates.io, or wait for it to expire before the job starts.

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
- [ ] Workspace crate names verified available before next release
