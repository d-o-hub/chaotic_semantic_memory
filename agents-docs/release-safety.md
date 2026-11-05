# Release Safety Requirements

**CRITICAL: Never release with failing CI. The release workflow is gated on the CI workflow finishing successfully on `main` (`workflow_run` on `ci.yml`), and a manual dispatch re-verifies CI green for HEAD before doing anything.**

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
   runner disk-full). Query the per-job conclusions before treating a green gate as a verified one. The same
   condition guards `release.yml`'s own `validate` (`:34-38`): that completion produced a green gate
   (41186134728) *and* a green release run (41186134783) that published nothing — **d-o-hub#969**, unfixed.
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
   release. A `recover=true` dispatch is **not** gated on the current queue — see Recovery item 3. Recipe:
   `github-ci-guardrails` → "The queue gate".

## Version Bump Workflow
1. Update `Cargo.toml` version
2. Update `wasm/package.json` version
3. Update `CHANGELOG.md` with new section
4. Run `cargo build --release` to sync Cargo.lock
5. Commit all version files together (atomic)
6. Push and wait for CI to pass on `main`; its completion triggers `release.yml`, which applies the version/tag check and owns the tag
7. Dispatch `publish-crates.yml` for the tag `release.yml` just created (`ref=refs/tags/v<version>`,
   `version=<version>`). Releases are no longer proof of a crates.io publish — and `verify-release`
   still probes the nine index paths, so until that dispatch lands the release run reports a **red**
   `verify-release` with a `Crates.io not published` annotation. That is the intended signal, not a
   flake: re-run the job after the dispatch and it goes green.

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
   A reviewer on that environment would hold the job *mid-publish*, after some crates landed.
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
3. **Recovery dispatch**: `gh workflow run release.yml -f recover=true -f recovery_tag=v0.3.9` retries missing
   artifacts for an existing tag idempotently. `recovery_tag` is a separate input and is required whenever
   `recover=true` — an empty one fails at `release.yml:53-56` with `recovery_tag is required when recover=true`.
   **It is gated on history, not on today's queue**: `recover=true` resolves the SHA with
   `git rev-list -n 1 <recovery_tag>` (the *tagged* commit), while a `workflow_run` release or a plain dispatch
   resolves `github.event.workflow_run.head_sha || github.sha` (the current head). Both then require the newest
   `CI` run and the newest `Pre-Release Gate` run **for that SHA** to be `completed/success`. Measured for
   `v0.3.9` → `642238f0`: CI 41104878082 and gate 41110522706, both green since 2026-10-30, so a recovery needs
   no drain. The hazard is the reverse one: re-running the tagged commit's CI creates a **new** gate run for
   it, and that run re-reads the open-PR list — so never re-run it while PRs are open, or a green historical
   verdict is replaced by a queue-dependent refusal.
   **It does not cover crates.io.** `release.yml` cannot mint a crates.io OIDC token (that event is
   `workflow_run`, which crates.io refuses — ADR-0104), so this recovery retries the GitHub Release
   and both npm channels only. The crate half of a recovery is a separate
   `gh workflow run publish-crates.yml -f ref=refs/tags/v0.3.9 -f version=0.3.9` dispatch.
