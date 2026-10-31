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
   its `status`/`conclusion` alone, while every real gate job is conditioned on
   `github.event.workflow_run.event == 'push'` (`pre-release-gate.yml:27-30`) — so a gate run spawned by any
   other event skips all six checks and still reports `success` (measured: 41186134728, 1 of 7 jobs ran).
   Query the per-job conclusions before treating a green gate as a verified one.
2. **Ensure Cargo.lock is synchronized**: `cargo build --release && git add Cargo.lock`
3. **Check existing releases**: `gh release list --limit 5`
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

## Platform-Specific Notes
- **macOS arm64**: NEON SIMD intrinsics require explicit unsafe blocks
- **Windows x64**: CI uses `--locked` flag, Cargo.lock must match Cargo.toml
- **WASM**: Size gate checks library (~870KB), not CLI binary (~5KB)

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
