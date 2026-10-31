---
name: release-management
description: GitHub release management, crates.io trusted publishing, npm provenance, and GitHub Pages documentation. Use when creating releases, publishing packages, or deploying docs.
---

# Release Management

Automated release pipeline: version sync, Trusted Publishing (crates.io/npm), and docs. **Main is protected** — never push release commits directly to `main`.

## Hard rules

1. **Never push directly to `main`.** Use branch → PR → required CI green → merge.
2. **Never create release tags manually for routine releases.** Tag owner is `.github/workflows/release.yml` (job `validate` / step “Ensure release tag exists”).
3. **Same-SHA release gate:** `release.yml` is triggered by successful completion of `CI` on `main`, resolves that run's exact `head_sha` (step `target_sha`), and refuses to tag or publish unless **both** the `CI` run and the `Pre-Release Gate` run for that same SHA are `completed/success`. The gate is a blocker checked inside the release job, not the release trigger: `pre-release-gate.yml` starts only after CI, so `validate` **waits** for a verdict rather than reading once — `GATE_DEADLINE = now + 1800`, `sleep 60`, break on `status == completed`, then still require `conclusion == success` (`release.yml:119-160`). It is fail-closed on timeout and on any non-success, so a red gate never publishes; do not re-dispatch while a verdict is pending. A manual dispatch, and `recover` with a `recovery_tag`, resolve a SHA and run the same check. The refusal **names the blocking job**: `scripts/gate-failure-detail.sh` turns the gate run's per-job conclusions into one `SUMMARY:` line inside the `::error::` annotation (`2 of 7 gate jobs failed: pr-drain-check, gate-summary`), which is how you tell a queue-policy block from a quality failure without leaving the release log — and a **jobless** gate run says so, because that is a rejected workflow file (rule 4), not a red check.
4. **A failed release run with no jobs is a bad workflow file, not a bad release:** `gh run view <id> --json jobs` returning `[]` means GitHub rejected the file before scheduling (since 2026-10-22 this is how `release.yml` failed: double-quoted / unquoted string literals in `if:` expressions — expressions take single quotes only). There is no step log, so read the YAML and run `scripts/check-actionlint.sh`; nothing published and no tag was created.
4. Identify the artifact channel first via the `dist-channel-selection` skill.

## Protected release flow

```
feature branch → PR → CI green → merge to main
        → CI completes for the main SHA → release.yml starts
        → release.yml verifies CI + Pre-Release Gate for that exact SHA
        → checks out that SHA → create tag v* → publish
```
`release.yml` owns the version tag and library/package publication. `dist.yml` has a single `workflow_dispatch` trigger and opens with a `disabled-notice` job: a tag created inside `release.yml` carries `GITHUB_TOKEN`, and Actions does not fire a tag-push trigger for a workflow from the same token, so platform CLI binaries are built by a manual `gh workflow run dist.yml` when a release needs them. `dist.yml` never creates a second tag or publishes crates/npm.

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

Run `./scripts/sync-version.sh <version>` BEFORE manually editing `Cargo.toml`. It updates Cargo.lock, CHANGELOG date section, README/docs minor pins, `wasm/package.json`, tests/examples, and regenerates llms files as configured.

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

If a name is taken by an unrelated project (e.g., `csm-core` was taken by Sesame CSM-1B TTS), rename the crate before publishing. Use prefixes like `chaotic-semantic-memory-*` or suffixes like `csm-core-lib` to avoid conflicts.

**Known conflicts (as of 2026-07-22):**
- `csm-core`: Taken by Sesame CSM-1B TTS project → renamed to `csm-core-lib`
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

## Post-publish verification (the half the workflow cannot prove)

`release.yml`'s own `verify-release` job probes **3** artifacts (root crate, two npm dist-tags, the release
object). The full set is 12 — 9 crates.io + 2 npm + the GitHub release — so a green tick there is weak
evidence about the other 8, and it accumulates errors under a default `bash -e {0}` shell that aborts at the
first miss. Count the artifacts from the **owning job** (every `run:` block of `publish-crates`, not the
`CRATES=` list: the root and `csm-duckdb` publish in later steps, `csm-duckdb` last because it dev-depends
on the root and polls the index for it).

Rules that came out of validating the 0.3.9 scripts against the *previous* release:

- **A green `Release` run is not proof that a release happened.** `validate` skips unless the trigger was a
  successful CI push (`release.yml:34-38`), and every dependent job then reads a null
  `needs.validate.outputs.release-needed` and skips too — so the run can conclude `success` having published
  nothing. Measured: run **41186134783** (2026-10-31T08:49Z) had all nine jobs skipped, only `notify`
  (`if: always()`) green, and crates.io still at 0/9. Read the job list, then the registry (#969).
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
- **Probe the crates.io token before the publish loop, not during it** (measured on v0.3.9, run 41110522627:
  nine `403 authentication failed` uploads ≈142 s apart, because each crate only fails at *its own* upload
  after a full verify build). `curl -H "Authorization: api-token $T" https://crates.io/api/v1/me` answers
  **200** (`.user.login` proves *which* identity), **401** malformed, **403** absent-or-revoked. Note the
  scope trap: a job declaring `environment: crates.io` uses the **environment** secret, which takes
  precedence over a differing repo-level copy.
- **Do not chase the `E0432` you see in a later crate's verify build.** `cargo publish` verifies in an
  isolated build that resolves companions from the **registry**, so when the root crate's upload 403s, the
  next crate rebuilds against the *previous* published version and fails on symbols that exist in the tree.
  It reads like a feature-gate defect and is a consequence of the credential.
- If anything after the tag exists, recover with `workflow_dispatch` `recover=true` + `recovery_tag=v*`
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
