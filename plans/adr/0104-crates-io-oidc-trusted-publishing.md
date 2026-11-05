# ADR-0104: crates.io OIDC Trusted Publishing From a Dispatch Workflow

## Status

Accepted

## Context

`v0.3.9` shipped to GitHub and to npm on both scopes, and never reached crates.io.
Measured against the sparse index (paths from `scripts/crates-io-index-path.sh`),
all nine published crates still reported `latest = 0.3.8` with zero rows for
`0.3.9`, while `@d-o-hub/csm` and `@d-o-hub/chaotic_semantic_memory` both had
`dist-tags.latest = 0.3.9`. The gap was nine `cargo publish` calls, not a release.

The reason is the credential model, not a missed step. Issue **#967** recorded nine
`403 authentication failed` uploads made with the repository secret
`CARGO_REGISTRY_TOKEN`, a long-lived crates.io API token created 2026-02-19 and last
updated 2026-07-22 — past crates.io's 90-day default validity, so any recovery
dispatch through that path was going to fail the same way again. ADR-0039 has listed
"Trusted Publishing for crates.io (OIDC-based, no tokens)" as a delivered item since
it was backfilled; ADR-0039's own positive consequence "No long-lived API tokens"
had never been true, and `book/src/release.md:123` asserted it anyway.

Reading the crates.io source and RFC 3691 ruled out the two obvious designs:

- **`workflow_run` cannot mint a crates.io token.**
  `crates_io_api::handlers::github::handle_github_token_inner` refuses OIDC assertions
  for the `workflow_run` and `pull_request_target` events. `release.yml` is triggered by
  `workflow_run: CI` (`release.yml:4-7`), so no edit to a job inside `release.yml` could
  ever have used Trusted Publishing. The trigger had to move; moving the job would not.
- **A tag trigger would be dead on arrival.** `release.yml` pushes the release tag with
  `secrets.GITHUB_TOKEN` (`release.yml:250`), and a `GITHUB_TOKEN` push does not spawn a
  workflow run. A `push: tags` publish workflow would therefore never fire for a release
  this repository just made.
- **The token probe in #967 could not pass.** crates.io serves `/api/v1/me` behind
  `AuthCheck::only_cookie()`, so the step that curled it and required HTTP 200 was
  testing something a token can never satisfy. It survived because
  `scripts/test-verify-release-step.sh` stubbed that exact response — a fixture was
  protecting dead code rather than testing behavior.

Two further properties shape the workflow itself. Trusted-publisher configs are created
only in the crates.io web UI, and there is no read-only API to enumerate them. And the
publish token crates.io mints is scoped to the *subset* of crates whose configs match the
assertion, so a missing config does not fail the run early: it 403s one crate and the
other eight land. That is a **partial publish**, the failure mode a single publishing job
is structurally blind to.

## Decision

1. **crates.io publishing leaves `release.yml` and becomes
   `.github/workflows/publish-crates.yml`**, triggered by `workflow_dispatch` only, with
   inputs `ref`, `version` and `dry_run`. A human names the ref and the version; that is
   the honest trigger given the two ruled-out options above.
2. **The OIDC exchange is `rust-lang/crates-io-auth-action` pinned to a SHA**
   (`c6f97d42243bad5fab37ca0427f495c86d5b1a18` # v1.0.5), which also corrects ADR-0042's
   claim that no such action exists. Job permissions are exactly `contents: read` +
   `id-token: write`, on top of a repository-wide `permissions: {}`.
3. **No secret is referenced.** Every step that publishes reads
   `CARGO_REGISTRY_TOKEN` from the action's output. `secrets.CARGO_REGISTRY_TOKEN`
   appears zero times in the file, and an empty-token guard replaces the dead
   `/api/v1/me` probe.
4. **Dispatch input reaches the shell only through `env`**, never spliced into a `run:`
   body, and the ref is shape-validated before use. The fixture asserts that no `run:`
   body in the file contains `${{`.
5. **`timeout-minutes: 25`** sits under crates.io's `ACCESS_TOKEN_LIFETIME` of 30 minutes,
   so an over-long run dies on the job rather than mid-upload with a 403 the publish loop
   would then misreport as a credential problem.
6. **Publish order and idempotency are carried over verbatim** from the job #971 fixed:
   seven members, then the root crate, then `csm-duckdb` (which needs the root visible in
   the index first), each skipped when the index already has the version.
7. **A final reconciliation step re-probes all nine crates** at the dispatched version and
   reds the run on any gap, naming what is missing. Because the loop is idempotent, the
   remedy it prints is to fix the config and re-dispatch — which makes a partial publish
   resumable instead of silent. Under `dry_run` it is skipped, since nothing was published.
8. **`release.yml` keeps GitHub Release and npm**, which is all `workflow_run` can
   legitimately prove. Its `verify-release` still probes the nine index paths and now emits
   an annotation pointing at the workflow to dispatch.

## Consequences

- A normal release run no longer proves crates.io. `verify-release`'s nine index probes
  come from `publish-crates`'s dependency, so deleting that job moves crates.io proof into
  `publish-crates.yml`'s reconciliation step. A release that skips the dispatch is now a
  visible gap in the release verdict rather than a green tick from a job that never ran.
- Publishing is a human action with a named ref, not a side effect of a green CI. That is
  the cost of crates.io refusing `workflow_run`; it is also what makes the v0.3.9 recovery
  and the migration proof the same dispatch.
- Nine configs must exist in the crates.io UI, each with owner `d-o-hub`, repository
  `chaotic_semantic_memory`, workflow filename `publish-crates.yml` and environment exactly
  `crates.io`. A gap is caught by reconciliation, not before the first upload.
- `dry_run` does **not** prove the configs: `cargo publish --dry-run` never performs the
  OIDC exchange. It proves packaging and version agreement only, and the docs say so.
- The `crates.io` environment stays without protection rules; reviewers on that
  environment would stall a dispatch mid-publish, after some crates had already landed.
- Revoking the crates.io API token is a crates.io-side action; deleting the GitHub secret
  does not revoke it. `vars.CRATES_IO_EXPECTED_LOGIN` is left in place deliberately —
  removing repository configuration is a separate decision from removing its last use.
- `scripts/test-verify-release-step.sh` grew structural assertions over the new file
  (permission set, single pinned auth action, auth step ordering before every token
  consumer, zero secret references, timeout bound, absent forbidden triggers) plus
  behavioral replays for the empty-token guard and the 2-of-9 partial publish.

## References

- ADR-0039 (Release Engineering — the undelivered OIDC item this closes)
- ADR-0042 (Release Automation — its "no such action" finding is superseded here)
- issue #967 (nine 403s), #971 (sparse-index quote splice)
- `.github/workflows/publish-crates.yml`, `.github/workflows/release.yml`
- `agents-docs/release-safety.md`, `book/src/release.md`
- RFC 3691 (trusted publishing for crates.io)
