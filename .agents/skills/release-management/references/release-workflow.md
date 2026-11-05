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

`recover=true` sets `release-needed=true`, and `release.yml`'s `trigger-crates-io-publish` runs
on that condition — so **one** dispatch now covers all three channels, crates.io included:

```bash
# GitHub Release + npm: retry missing artifacts for an existing tag
gh workflow run release.yml --ref main -f recover=true -f recovery_tag=v0.3.9
# → the same run dispatches publish-crates.yml for refs/tags/v0.3.9 and waits for its verdict
```

Dispatch the crate workflow by hand when the release half is already verified and only the
crates are missing, or when you want `dry_run` (a release run never asks for one):

```bash
# crates.io: publish the same tag over OIDC, no secret involved
gh workflow run publish-crates.yml -f ref=refs/tags/v0.3.9 -f version=0.3.9
```

`recovery_tag` is not optional: with `recover=true` an empty tag makes `validate` fail with
`::error::recovery_tag is required when recover=true.` (`release.yml:53-57`). `ref` and
`version` are likewise both required on `publish-crates.yml`, and `version` must be the
bare `0.3.9` — the workflow cross-checks it against what the ref's manifests declare.
Neither path has been exercised end to end yet: v0.3.9's recovery is the first run through it.

The recovery workflow:
1. Sees tag exists, but `recover=true` → proceeds with `release-needed=true`
2. Each job checks if its artifact already exists → skips if so
3. Publishes only missing artifacts (idempotent) — the crate loop skips by index row, the
   dispatch job skips nothing it did not already see published

**Prerequisites for recovery:**
- Nine **crates.io trusted-publisher configs** exist (owner `d-o-hub`, repository
  `chaotic_semantic_memory`, workflow filename `publish-crates.yml`, environment exactly
  `crates.io`) — see `trusted-publishing.md`. No `CARGO_REGISTRY_TOKEN` is read by any
  workflow: #967's nine 403s came from that expired long-lived secret, and OIDC is what
  replaced it. `publish-crates.yml`'s final step re-probes the sparse index for all nine and
  reds the run on any gap, so a missing config shows up as a named missing crate rather than
  a green run with eight of nine published. A `dry_run=true` dispatch does **not** prove the
  configs — it never performs the exchange.
- The tagged commit still has a green history: `recover=true` gates on the **newest** `CI` and
  `Pre-Release Gate` runs *for the commit the tag points at* (`release.yml:94`), not on the current head, so
  an open-PR queue does **not** block a recovery. Verify before promising either — for `v0.3.9`
  (`642238f0`) both are green (41104878082 / 41110522706). Re-running that commit's CI is the way to spoil it:
  the new gate run re-reads today's open-PR list and can refuse where history passed
- `publish-crates.yml` has no CI gate of its own: it publishes whatever `ref` you name, so
  naming a ref whose manifests disagree with `version` is the one thing that stops it. From
  ADR-0104 a release names the ref for you, which makes that cross-check the only thing
  between a mis-tagged release and nine `cargo publish` calls.
- Workspace crate names must not be taken by unrelated projects (see below)

## crates.io name conflicts

Workspace crate names may be taken by unrelated projects on crates.io. If so, `cargo publish` will fail with:

```
403 Forbidden: this crate exists but you don't seem to be an owner.
```

**Solution:** Rename crates before first publish. Use prefixes like `chaotic-semantic-memory-*` or suffixes like `-lib` to avoid conflicts.

```bash
# Check if names are available
for crate in csm-chaos csm-core-lib csm-traits csm-embedding csm-memory csm-retrieval csm-persistence \
  chaotic_semantic_memory csm-duckdb; do
  echo -n "$crate: "
  cargo search "$crate" 2>/dev/null | head -1
done
```

**Known conflicts (as of 2026-07-22):**
- `csm-core`: Taken by Sesame CSM-1B TTS project → renamed to `csm-core-lib`

## Environment protection (measured 2026-10-31: there is no wait timer here)

This section previously asserted a "default 15 minutes" wait timer on the `crates.io` environment and told
you to remove it. Measured on this repo, `crates.io`, `npm` and `npm-cli` all report `protection_rules: []`;
only `github-pages` carries a rule (`branch_policy`). Nothing can stall `publish-crates.yml` behind an approval
or a timer, so a failed publish has a real cause — on v0.3.9 it was nine `403 authentication failed` uploads
from a dead `CARGO_REGISTRY_TOKEN` (issue #967), not a waiting deployment. Keep it that way: reviewers on the
`crates.io` environment would park the job *between* crate uploads, which is how a partial publish is made.
Since ADR-0104 the stall is no longer invisible: `release.yml`'s `trigger-crates-io-publish` waits that run
out (38 minutes of waiting inside its 40-minute job) and goes red with `parked on an environment approval`, so
adding protection turns every release into a partially published one you have to notice.

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
