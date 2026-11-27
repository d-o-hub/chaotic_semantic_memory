# Release Safety Requirements

**CRITICAL: Never release with failing CI. `release.yml` is triggered by a `v*` tag push (or a tagged `workflow_dispatch`), and its `validate` job refuses to publish unless **both** the `CI` run and the `Pre-Release Gate` run for the exact tagged commit are `completed/success`. Tags are cut by `auto-release.yml`'s `ship` job (which only tags a `main` tip whose CI just went green), or by a human — never by `release.yml` itself.**

## Artifact Channels
- **Rust Library:** `chaotic_semantic_memory` (crates.io / cargo)
- **JS/WASM Library:** `@d-o-hub/chaotic_semantic_memory` (npm WASM)
- **CLI Tool:** `@d-o-hub/csm` (npm CLI)
Refer to `.agents/skills/dist-channel-selection/SKILL.md` for canonical commands.

## Pre-Release Checklist (MANDATORY)
1. **Verify CI passes on all platforms**: `gh run list --workflow=ci.yml --limit 3`. Read the *gate* run the
   same way but never at run level: `release.yml` accepts the **newest** `pre-release-gate` run for the SHA on
   its `status`/`conclusion` alone, while every real gate job requires
   `workflow_run.event == 'push'` **and** `workflow_run.conclusion == 'success'`
   (`pre-release-gate.yml:27-30`) — so a gate run spawned by a CI attempt that failed skips all six checks and
   still reports `success` (measured: 41186134728, 1 of 7 jobs ran, behind it CI 41184146253 attempt 1 dying on
   runner disk-full). Query the per-job conclusions before treating a green gate as a verified one. `release.yml`'s
   `validate` applies the same standard to the commit the tag names: it resolves that SHA (`git rev-list -n 1
   <tag>` for a tagged dispatch, `github.sha` for a tag push), then requires the newest `CI` run and the newest
   `Pre-Release Gate` run **for that SHA** to be `completed/success` — the gate read is a bounded poll
   (`GATE_DEADLINE = now + 1800`, `sleep 60`), fail-closed on timeout and on any non-success, and the refusal
   names the blocking jobs via `scripts/gate-failure-detail.sh`. A green release run that published nothing is
   still not publish evidence — **d-o-hub#969** — read `verify-release`'s rows or the registries.
2. **Ensure Cargo.lock is synchronized**: `cargo build --release && git add Cargo.lock`
3. **Check existing releases**: `gh release list --limit 5`. A green `release.yml` run is **not** publish
   evidence — when `v{version}` already exists `validate` sets `release-needed=false`, every downstream job
   skips, and the run still reports `success` (measured: 41195178865 on `14687660`, green and silent). Read
   `verify-release`'s per-artifact rows or query the registries directly — with the index path built
   correctly: the sparse index shards by name length (`1/<n>`, `2/<n>`, `3/<first>/<n>`, else
   `<first2>/<next2>/<n>`, so `cs/m-/csm-chaos` and `ch/ao/chaotic_semantic_memory`). A hand-built path
   returns S3 `NoSuchKey`, which reads as "never published" for a crate that is on the registry — measured
   by getting exactly that false answer for `csm-chaos` before fixing the shard.
4. **Validate changelog entry exists**: `grep -q "^## \[${VERSION}\]" CHANGELOG.md`
5. **Verify crates.io availability (first publish)**: check workspace names via `cargo search`
6. **Drain the PR queue (normal releases only)**: the gate's `pr-drain-check` job fails while **any** open
   PR lacks a `hold-for-next`/`deferred` label, which refuses the release for that head (`gh pr list --state
   open --json number,title,labels`). Label a PR you intend to keep open rather than merging it to unblock a
   release. A tagged recovery dispatch is **not** gated on the current queue — see Recovery item 3. Recipe:
   `github-ci-guardrails` → "The queue gate".

## Version Bump Workflow

**Automatic path (`auto-release.yml`)** — after green CI on a `main` push, `auto-release.yml`'s `prepare`
job derives the bump from the conventional commits since the last release tag
(`scripts/release-derive.sh next`: threshold of 3 release-worthy commits, `feat!`/`BREAKING CHANGE` →
major at ≥1.0 / minor at 0.x, `feat` → minor, `fix`/`perf` → patch, nonconventional → patch,
`Rename-Confirmed:` trailer promotes a rename to breaking, unconfirmed shipped-surface rename →
`held-rename`) and opens or refreshes a `release/vX.Y.Z` pull request seeded by
`release-derive.sh prepare` (`sync-version.sh` + changelog section + `verify-version-sync.sh`). A
propos­al the derivation no longer produces is closed as superseded — at most one automation-owned
proposal is open. Because the PR is pushed with `GITHUB_TOKEN`, its checks need a manual
`gh workflow run ci.yml -f ref=release/vX.Y.Z` (or a maintainer push) to run; a **maintainer merges it**
(squash). After the merge, `ship` (on the next green `main` CI, or via `gh workflow run
auto-release.yml` against `main`) tags the `main` tip `vX.Y.Z` and dispatches `release.yml -f tag=vX.Y.Z`.

**Manual path** — branch, `scripts/sync-version.sh <version>`, CHANGELOG section, PR, merge; then tag
the updated `main` tip (`git tag v<version> && git push origin v<version>`) and `release.yml` takes it,
or run `gh workflow run release.yml -f tag=v<version>`.

After the tag, `release.yml` runs end to end: `validate` (tag == root `[package].version`, CI + gate
green for the tagged commit, changelog, version sync) → build matrix → `create-github-release`
(SHA256SUMS + changelog notes) → `trigger-crates-io-publish`, which dispatches `publish-crates.yml`
(`ref=refs/tags/v<version>`, `version=<version>`) and waits for that run's verdict. Publishing lives in
`publish-crates.yml` because the crates.io trusted-publisher configs name **that** workflow file
(ADR-0104), so a red dispatch job means the *other* run went wrong; open it. `verify-release` probes
the nine index paths independently and stays red while any crate is missing, with a
`Crates.io not published` annotation. That is the intended signal, not a flake. A hand dispatch
(`gh workflow run publish-crates.yml -f ref=… -f version=…`) remains available, and is the only way to
reach `dry_run=true`.

## Platform-Specific Notes
- **macOS arm64**: NEON SIMD intrinsics require explicit unsafe blocks
- **Windows x64**: CI uses `--locked` flag, Cargo.lock must match Cargo.toml
- **WASM**: Size gate checks library (~870KB), not CLI binary (~5KB)

## crates.io Trusted Publishing (operator setup, one time)
1. For **each** of the nine crates — `csm-chaos`, `csm-core-lib`, `csm-traits`, `csm-embedding`,
   `csm-memory`, `csm-retrieval`, `csm-persistence`, `chaotic_semantic_memory`, `csm-duckdb` — open
   that crate's *Trusted Publishing* page on crates.io and add a publisher with owner `d-o-hub`,
   repository `chaotic_semantic_memory`, workflow filename `publish-crates.yml`, and environment
   **exactly** `crates.io`. Nine crates means nine configs; one crate without a config is one
   failing publish, not one failing workflow.
2. The crates.io web UI is the only place these configs exist and there is no read-only endpoint
   that lists them, so a gap surfaces at `publish-crates.yml`'s reconciliation step — after the
   other crates have already uploaded. Nothing here can pre-flight it.
3. A gap therefore produces a **partial publish**: crates.io scopes the minted token to the subset
   of crates whose trusted publishers match the assertion. The publish loop is idempotent, so the
   recovery is to create the missing config and re-dispatch the same `ref` + `version`; crates
   already in the index at that version are skipped.
4. Keep the `crates.io` environment's `protection_rules` empty (measured `[]`, see Failure item 2).
   A reviewer on that environment would hold the job *mid-publish*, after some crates landed — and
   since ADR-0104 it also stalls the release run that dispatched it until that job's wait expires.
5. After the OIDC path is proven, revoke the crates.io API token **on crates.io**. Deleting the
   `CARGO_REGISTRY_TOKEN` GitHub secret does not revoke the credential; the token itself
   (created 2026-02-19, last updated 2026-07-22) is what #967's nine 403s were rejected against.

## Common Deployment Failures & Recovery
1. **crates.io name conflicts**: Workspace crate names may be taken. Rename before first publish (e.g., `csm-core` -> `csm-core-lib`).
2. **Environment protection — measure, don't assume.** This doc long claimed a "default 15 min wait timer
   on the crates.io environment"; measured 2026-10-31, `crates.io`, `npm` and `npm-cli` all have
   `protection_rules: []` here (only `github-pages` has a rule), so a pending deployment was never the cause
   of a stuck publish. Verify before diagnosing:
   `gh api repos/{owner}/{repo}/environments --jq '.environments[] | "\(.name) \(.protection_rules)"'`, then
   `gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments` — the second only answers when a
   rule can actually hold the job.
3. **Recovery dispatch**: `gh workflow run release.yml -f tag=v0.3.9` retries missing artifacts for an
   existing tag idempotently (it replaces the old `recover=true`/`recovery_tag` inputs).
   **It is gated on history, not on today's queue**: a tagged dispatch resolves the SHA with
   `git rev-list -n 1 <tag>` (the *tagged* commit), then requires the newest `CI` run and the newest
   `Pre-Release Gate` run **for that SHA** to be `completed/success`. Measured for `v0.3.9` → `642238f0`:
   CI 41104878082 and gate 41110522706, both green since 2026-10-30, so a recovery needs no drain. The
   hazard is the reverse one: re-running the tagged commit's CI creates a **new** gate run for it, and
   that run re-reads the open-PR list — so never re-run it while PRs are open, or a green historical
   verdict is replaced by a queue-dependent refusal.
   **It covers crates.io too.** The recovery run dispatches `publish-crates.yml` against the existing
   tag; the publish loop is idempotent, so crates already at that version are skipped. Every publish
   step is idempotent the same way: npm versions already published are skipped, and an existing GitHub
   Release has its assets replaced. A direct `gh workflow run publish-crates.yml
   -f ref=refs/tags/v0.3.9 -f version=0.3.9` remains available whenever you want to name the ref
   yourself or pass `dry_run`. An **untagged** dispatch (`gh workflow run release.yml`) is a dry run:
   it builds and packages every target but never publishes.
