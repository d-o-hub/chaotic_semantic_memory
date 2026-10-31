# Release Workflow (canonical)

Matches `.github/workflows/release.yml` as of Wave 32. Prefer the skill entrypoint `SKILL.md` for operator steps; this file holds detail and recovery.

## Trigger and tag owner

| Item | Value |
|------|-------|
| Workflow | `.github/workflows/release.yml` |
| Triggers | `workflow_run` on `CI` completion (success, branch `main`), `workflow_dispatch` |
| Tag owner | Workflow job `validate`, step **Ensure release tag exists** |
| Tag format | `v` + version from root `Cargo.toml` |

```
merge PR to main
  → CI completes successfully on main
  → release.yml starts (workflow_run; no polling, no wait ceiling)
  → validate: changelog, sync-version clean, package list
  → git tag v$VERSION && git push origin v$VERSION   # ONLY tag owner
  → build matrix / publish crates / npm / GitHub release
```

A `workflow_dispatch` run skips the event gate but must see a green CI run for
HEAD (single fail-closed query) before validating; the job-level `if` only
releases when the upstream CI run concluded `success` in this repository.

**Humans do not create routine `v*` tags.** Manual tag push is recovery-only (below).

## Protected-main policy

1. Create `release/vX.Y.Z` (or `chore/release-…`) from updated `main`
2. Run `scripts/sync-version.sh <version>` BEFORE editing manifests manually, then fix CHANGELOG and complete version changes
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
# Retry missing artifacts for an existing tag
gh workflow run release.yml --ref main -f recover=true -f recovery_tag=v0.3.9
```

`recovery_tag` is not optional: with `recover=true` an empty tag makes `validate` fail with
`::error::recovery_tag is required when recover=true.` (`release.yml:53-56`).

The recovery workflow:
1. Sees tag exists, but `recover=true` → proceeds with `release-needed=true`
2. Each job checks if its artifact already exists → skips if so
3. Publishes only missing artifacts (idempotent)

**Prerequisites for recovery:**
- `CARGO_REGISTRY_TOKEN` must belong to an owner of ALL workspace crates on crates.io, and must be written to
  **both** scopes that can supply it (the repo secret and the `crates.io` environment secret) — `publish-crates`
  reads the environment copy
- The tagged commit still has a green history: `recover=true` gates on the **newest** `CI` and
  `Pre-Release Gate` runs *for the commit the tag points at* (`release.yml:60`), not on the current head, so
  an open-PR queue does **not** block a recovery. Verify before promising either — for `v0.3.9`
  (`642238f0`) both are green (41104878082 / 41110522706). Re-running that commit's CI is the way to spoil it:
  the new gate run re-reads today's open-PR list and can refuse where history passed
- Workspace crate names must not be taken by unrelated projects (see below)

## crates.io name conflicts

Workspace crate names may be taken by unrelated projects on crates.io. If so, `cargo publish` will fail with:

```
403 Forbidden: this crate exists but you don't seem to be an owner.
```

**Solution:** Rename crates before first publish. Use prefixes like `chaotic-semantic-memory-*` or suffixes like `-lib` to avoid conflicts.

```bash
# Check if names are available
for crate in csm-chaos csm-core-lib csm-traits csm-embedding csm-memory csm-retrieval csm-persistence; do
  echo -n "$crate: "
  cargo search "$crate" 2>/dev/null | head -1
done
```

**Known conflicts (as of 2026-07-22):**
- `csm-core`: Taken by Sesame CSM-1B TTS project → renamed to `csm-core-lib`

## Environment protection (measured 2026-10-31: there is no wait timer here)

This section previously asserted a "default 15 minutes" wait timer on the `crates.io` environment and told
you to remove it. Measured on this repo, `crates.io`, `npm` and `npm-cli` all report `protection_rules: []`;
only `github-pages` carries a rule (`branch_policy`). Nothing can stall `publish-crates` behind an approval
or a timer, so a failed publish has a real cause — on v0.3.9 it was nine `403 authentication failed` uploads
from a dead `CARGO_REGISTRY_TOKEN` (issue #967), not a waiting deployment.

```bash
gh api repos/{owner}/{repo}/environments \
  --jq '.environments[] | "\(.name) \([.protection_rules[].type])"'
gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments   # [] when nothing waits (top-level array)
```

**If** protection is ever added to a publish environment, remove it in GitHub Settings → Environments or wait
it out — but query first: an ungated run answers `[]`, and reading that as "the cause" is how a wrong
diagnosis survives a release.

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
