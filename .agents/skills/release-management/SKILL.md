---
name: release-management
description: GitHub release management, crates.io trusted publishing, npm provenance, and GitHub Pages documentation. Use when creating releases, publishing packages, or deploying docs.
---

# Release Management

Automated release pipeline: version sync, Trusted Publishing (crates.io/npm), and docs. **Main is protected** — never push release commits directly to `main`.

## Hard rules

1. **Never push directly to `main`.** Use branch → PR → required CI green → merge.
2. **`release.yml` never creates release tags.** Tag owner is `auto-release.yml`'s `ship` job (tags the verified `main` tip and dispatches `release.yml -f tag=v*`), or a human push. `release.yml`'s `validate` only asserts the tag exists and matches the root `[package].version`.
3. **Same-SHA release gate:** `release.yml` is triggered by a `v*` tag push (or a tagged `workflow_dispatch`), resolves the commit the tag names (step `target_sha` — `git rev-list -n 1 <tag>` for a dispatch, `github.sha` for a push), and refuses to publish unless **both** the `CI` run and the `Pre-Release Gate` run for that same SHA are `completed/success`. The gate is a blocker checked inside `validate`, not the release trigger: `pre-release-gate.yml` starts only after CI, so `validate` **waits** for a verdict rather than reading once — `GATE_DEADLINE = now + 1800`, `sleep 60`, break on `status == completed`, then still require `conclusion == success` (the “Verify CI and Pre-Release Gate are green for target commit” step). It is fail-closed on timeout and on any non-success, so a red gate never publishes; do not re-dispatch while a verdict is pending. The refusal **names the blocking job**: `scripts/gate-failure-detail.sh` turns the gate run's per-job conclusions into one `SUMMARY:` line inside the `::error::` annotation (`2 of 7 gate jobs failed: pr-drain-check, gate-summary`), which is how you tell a queue-policy block from a quality failure without leaving the release log — and a **jobless** gate run says so, because that is a rejected workflow file (rule 4), not a red check. `pr-drain-check` is queue policy, not quality: it fails while **any** open PR lacks a `hold-for-next`/`deferred` label, so draining or labelling the queue is a step of releasing (measured 2026-10-31: run 41188203137 refused `bfa15d49` with `2 of 7 gate jobs failed: pr-drain-check, gate-summary`, no tag). **A tagged recovery dispatch is exempt from today's queue** — it resolves the *tagged* commit and reads that commit's historical gate run, so `v0.3.9`'s recovery needs no drain (CI 41104878082 + gate 41110522706, both green); re-running that commit's CI is what would spoil it, by spawning a fresh gate run against today's queue. Sequence: `github-ci-guardrails`.
4. **A failed release run with no jobs is a bad workflow file, not a bad release:** `gh run view <id> --json jobs` returning `[]` means GitHub rejected the file before scheduling (since 2026-10-22 this is how `release.yml` failed: double-quoted / unquoted string literals in `if:` expressions — expressions take single quotes only). There is no step log, so read the YAML and run `scripts/check-actionlint.sh`; nothing published and no tag was created.
4. Identify the artifact channel first via the `dist-channel-selection` skill.

## Protected release flow

```
feature branch → PR → CI green → merge to main
        → auto-release.yml: green CI on main → release/vX.Y.Z PR (derived bump)
        → maintainer merges → ship tags the main tip v* → dispatches release.yml
        → release.yml verifies CI + Pre-Release Gate for the tagged commit
        → build matrix → GitHub Release → crates.io dispatch → npm publishes
```
`auto-release.yml` owns version-bump PRs and tag creation; `release.yml` owns publication behind an existing tag. `dist.yml` has a single `workflow_dispatch` trigger and opens with a `disabled-notice` job: a tag pushed by `ship` carries `GITHUB_TOKEN`, and Actions does not fire a tag-push trigger for a workflow from the same token, so platform CLI binaries are built by a manual `gh workflow run dist.yml` when a release needs them. `dist.yml` never creates a second tag or publishes crates/npm.

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

# 5. Commit only the intended release files on the branch — never on main
git add Cargo.toml Cargo.lock CHANGELOG.md VERSION wasm/package.json cli-npm/package.json
git commit -m "release: v0.2.0"

# 6. PR → trigger its checks (GITHUB_TOKEN-pushed PRs start "approval required") → merge
#    (squash preferred; no --auto multi-PR loops)
git push -u origin HEAD
gh pr create --title "release: v0.2.0" --body "Version bump and changelog for 0.2.0"
gh workflow run ci.yml -f ref=release/v0.2.0   # or push once as a maintainer
gh pr checks --watch
gh pr merge --squash

# 7. After merge: tag the updated main tip, or let auto-release.yml's ship do it
gh workflow run auto-release.yml --ref main    # ship asserts main tip + derives
#    or: git tag v0.2.0 && git push origin v0.2.0
gh run list --workflow=release.yml --limit 3
gh run watch
```

**Do not** `git push origin main`. Tagging the merged `main` tip is the release trigger; `auto-release.yml` automates it, and a human may do the same by hand.

## Version sync (before PR)

Run `./scripts/sync-version.sh <version>` BEFORE manually editing `Cargo.toml`. It updates Cargo.lock, CHANGELOG date section, README/docs minor pins, `wasm/package.json`, cli-npm, every workspace member's own version and — since #964 — **every** intra-workspace dependency pin via `scripts/workspace-version-edges.py`. Its docs/tests/examples sweep is still literal-matched to the *previous* version, so strings older than that survive it (#982).

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
| crates.io | OIDC Trusted Publishing | `release.yml` dispatches `publish-crates.yml` and waits; hand dispatch for recovery or `dry_run` |
| npm packages | `npm publish --provenance` / OIDC | Same |
| GitHub Release | `softprops/action-gh-release` | Same |
| GitHub Pages | mdBook deploy | Docs path on main / release jobs |

## crates.io name verification (FIRST PUBLISH)

Before a first publish, check all **nine** workspace names against crates.io in one loop. The command, the
conflict history (`csm-core` → `csm-core-lib`) and the measured current state — every one of the nine is
already owned by this repository and publishes through 0.3.8, while `csm-wasm` has no crates.io row at all —
live in `references/release-workflow.md`, section "crates.io name conflicts". Do not fork that list here; two
copies of a nine-item list drift.

## GitHub environment configuration

**Measure the environments instead of trusting prose.** This section used to assert a "default 15 min wait
timer" on `crates.io`; measured 2026-10-31, `crates.io`, `npm` and `npm-cli` all report
`protection_rules: []` (no wait timer, no required reviewer) and only `github-pages` carries a rule — a
`branch_policy`. So a stalled or failed publish is *not* an environment approval:

```bash
gh api repos/{owner}/{repo}/environments --jq '.environments[] | "\(.name) \([.protection_rules[].type])"'
```

If a rule ever is added, `gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments` names the
deployment still waiting — run it to confirm a wait exists, not to explain a failure with another cause.

## Tag ownership (single owner)

| Who | Action |
|-----|--------|
| **Human / PR** | Bump `Cargo.toml`, changelog, sync-version, merge via PR |
| **`auto-release.yml`** | Green CI on `main` → derive bump → open/refresh `release/vX.Y.Z` PR → after merge, `ship` tags the main tip and dispatches `release.yml` |
| **`release.yml`** | Validates the tagged commit (tag == version, CI + gate green) → package → GitHub release + npm → dispatch the crates.io publish (never creates the tag, never mints a crates.io token itself) |
| **`publish-crates.yml`** | Dispatched by `release.yml`'s `trigger-crates-io-publish` with that release's `ref` + `version` (or by hand) → OIDC exchange → publish the nine crates |

If a required artifact already exists, the corresponding publish step skips it (idempotent). Manual tag push is now a *normal* trigger; recovery of a partial release is the tagged dispatch `gh workflow run release.yml -f tag=v*`, approval-gated — see `references/release-workflow.md`.

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

## Post-publish verification (the half the workflow cannot prove)

`verify-release` enumerates the full set — **9** crates.io index paths, both npm packages, and the expected
release assets (5 CLI targets → 7 files with `SHA256SUMS` and the WASM tarball, from the same
`scripts/release-cli-matrix.sh` output that drives the build) — and accumulates misses instead of aborting at
the first. It waits on `trigger-crates-io-publish`, so the crates it probes are the ones that workflow just
tried to publish: a crates.io gap is a *failure with a named job to open*, not an instruction to go dispatch
something by hand — re-driving the release with `gh workflow run release.yml -f tag=v*` is the recovery path
(the hand `publish-crates.yml` dispatch covers a crates-only gap, and an untagged `release.yml` dispatch is
the build-only dry run). Inside the
publishing path, the set is split across steps — seven members in one loop, then the root crate, then
`csm-duckdb` last because it path-depends on the root and polls the sparse index for it — so read every
`run:` block, not just the loop header, before claiming a crate was covered.

Rules that came out of validating the 0.3.9 scripts against the *previous* release:

- **A green `Release` run is not proof that a release happened.** Every publish job keys on
  `needs.validate.outputs.release-needed == 'true'`, which only a tag-carrying trigger (push of `v*` or a
  tagged dispatch) sets — an untagged dispatch is a dry run — so a mis-triggered run can conclude `success`
  having published nothing. Measured: run **41186134783** (2026-10-31T08:49Z, the old `workflow_run` trigger)
  had all nine jobs skipped, only `notify` (`if: always()`) green, and crates.io still at 0/9. Read the job
  list, then the registry (#969).
- **Run the control, don't hope for it.** A verifier is trusted only after one input that must pass **and**
  one that must fail produce those outputs. Most verifier bugs do not crash — they print a confident
  sentence about a different fact ("release missing" for a **lightweight** tag whose `.object.url` contains
  `commits/`, never `tags/`; "published without provenance" for a version that was never published).
- **Probe a tag with `GET /repos/{o}/{r}/git/matching-refs/tags/{t}`** and match `.ref`; `git/ref/{t}` +
  `.object.url` only answers for annotated tags.
- **Make the scratch dir disposable.** `gh release download` refuses to overwrite, so a re-run of an otherwise
  correct script fails its own asset check; `rm -rf "$WORK"` at the top, not EEXIST handling per call.
- **Pin the repository, not the cwd.** `gh …`/`git ls-remote` resolve from the current directory's remote; run
  from outside a checkout they emit a git error into the slot reserved for a verdict. Pass `--repo`/`git -C`
  and give the error branch its own label (`FAIL`, not `MISS`).
- Before a release, confirm the publish environments cannot stall: `protection_rules: []` means no wait timer
  and no required reviewer; mid-run use `…/actions/runs/{id}/pending_deployments`.
- **Intra-workspace version pins drift silently — run the edge audit, not the version grep.**
  `^0.3.8` accepts `0.3.9`, so a stale member pin resolves fine, every test passes, and
  `Cargo.lock` (which records resolved versions, not requirements) never churns. #964 measured
  18 of 26 pins stale across 11 manifests while `verify-version-sync.sh` printed
  `✓ All local versions synchronized`. `python3 scripts/workspace-version-edges.py --check`
  is the sensor; `--sync <ver>` is the writer, from the same parser.
  Detail: [references/version-pin-edges.md](references/version-pin-edges.md).
- **A crates.io credential cannot be probed; only the publish itself is evidence.** v0.3.9 burned nine
  `403 authentication failed` uploads ≈142 s apart (run 41110522627) because each crate only fails at *its
  own* upload, after a full verify build. The #967 fix tried to pre-flight that with
  `curl -H "Authorization: api-token $T" https://crates.io/api/v1/me` and require **200** — a probe that can
  never pass: crates.io serves `/api/v1/me` behind `AuthCheck::only_cookie()`, so no token, valid or dead,
  returns 200. It survived review because its own fixture stubbed the 200 (ADR-0104). What actually closes
  the window: guard on an **empty** exchange output, abort the loop on the first 401/403 instead of repeating
  a dead credential per crate, and re-probe the sparse index for all nine at the end and red the run on any
  gap. The related trap moved too: a token-based job declaring `environment: crates.io` silently used the
  *environment* secret over a differing repo copy; under OIDC the environment is a claim in the assertion, so
  the config must name environment `crates.io` exactly or that crate's exchange fails alone.
- **Do not chase the `E0432` you see in a later crate's verify build.** `cargo publish` verifies in an
  isolated build that resolves companions from the **registry**, so when the root crate's upload 403s, the
  next crate rebuilds against the *previous* published version and fails on symbols that exist in the tree.
  It reads like a feature-gate defect and is a consequence of the credential.
- If anything after the tag exists, recover with `gh workflow run release.yml -f tag=v*`
  — never delete or re-create the tag. The guards are idempotent, so a partial release is re-driven, not
  rebuilt.

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
