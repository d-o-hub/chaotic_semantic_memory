# v0.3.9 release validation (2026-10-30)

Standing runbook for the v0.3.9 publication: what was proven before the wave merged, what only the
`main` pipeline can prove, and the two claims in my own working notes that measurement reversed.
Static half measured on the wave head `e766a4d1` and re-checked on `main` at `4ac8422d`; every
registry number below was read live on 2026-10-30, not carried from a PR body. **Every `release.yml`
line number here is `git show 4ac8422d:…`** — #962 is net +37 lines (42 insertions, 5 deletions) with
the insertion above the publish steps, so the same landmarks were `:307`/`:342`/`:355`/`:584` on
`3953e818` and are `:344`/`:379`/`:392`/`:621` now (measured on both heads; three of my earlier npm
citations matched neither tree and are corrected below).

**Scope split.** Everything in §1–§5 is provable from a checkout and a registry query. §6 is the part
that cannot be: tag creation, the 11 publishes, asset download, install-from-registry.

## 1. Version sync — PASS on main, no bump commit needed

| Input | Value | Proof |
|---|---|---|
| `Cargo.toml:124` | `0.3.9` | grep |
| `crates/*/Cargo.toml` (10 members) | `0.3.9` each | grep |
| Root deps on members (7 distinct: chaos, core-lib, embedding, memory, retrieval, traits, persistence — the last also re-listed for `wasm32` with `features=["wasm"]`) | `version = "0.3.9"` on every edge | `sync-version.sh:57` rewrites them; 14 hits in `Cargo.lock` |
| `VERSION` (tracked) | `0.3.9` | `cat` |
| `wasm/package.json`, `cli-npm/package.json` | `0.3.9` | grep |
| `README.md` / `book/src/getting-started.md` | `{ version = "0.3" }` | correct: `sync-version.sh:61,63` pins **minor**, not patch |
| `CHANGELOG.md` | one dated header `## [0.3.9] - 2026-10-30` (`:8`, `grep -c` = 1) and one `[0.3.9]:` link row (`:498`) | the two `release.yml` `validate` failure modes |

`./scripts/verify-version-sync.sh` → rc 0, and the gate's own `version-triad` query re-run on the
post-merge tree agrees: `cargo=0.3.9 changelog=0.3.9 wasm=0.3.9`. The tag is created by `release.yml`
the first time it sees green CI **and** a green pre-release gate on a `main` head; no release branch
and no manual `git tag` (per `release-management`: the workflow owns the tag).

## 2. crates.io channel — content verified from a clean tree, publish graph sound

Package manifest built from `git archive HEAD` extracted to `/tmp/csm-pkgtest` (a dirty working tree is
not evidence — my own tree added 152 ignored files), plus `cargo package --list` re-measured on this
head: **root 255 files** (254 on `e766a4d1`, the +1 being this file), `csm-duckdb` 31, `csm-cli` 5,
`csm-wasm` 5, `--workspace` total **452**. My working notes carried a "603 files" figure that does not
reproduce under any of those invocations — it is deleted rather than explained, because the likeliest
source is a `--allow-dirty` run against a tree that still had its untracked residue, which is the same
reason the clean `git archive` extraction exists.

- **255 files in the root package** (118 `src`, 74 `tests`, 35 `examples`, 9 `benches`, 19 non-source),
  re-measured against the **published 0.3.8 crate downloaded from `static.crates.io` — 215 files**, whose
  14 non-source entries are cargo's 7 scaffolding files (`Cargo.lock`, `Cargo.toml`, `Cargo.toml.orig`,
  `.cargo_vcs_info.json`, `README.md`, `LICENSE`, `CHANGELOG.md`) plus **7** nested README/LICENSE files
  (`plans/README.md`, `plans/handoffs/README.md`, `progress/mutation/README.md`,
  `docs/architecture/README.md`, `cli-npm/{README,LICENSE}`, `wasm/README.md`) that cargo includes
  unconditionally once `[package] include` reaches those directories. This wave adds **5** more, all
  `plans/evidence/*/README.md` measurement write-ups and no credentials (grepped
  `token|secret|api_key|password|ghp_` — only prose about the encoder's *token* fast path). My notes had
  said "8 extras, verified by diffing prefix lists"; the extras count was one high and the diff was never
  run, so both numbers above come from `tar tzf` on the downloaded artifact.
- **`VERSION` is absent from the tarball and that is safe**: `include` omits it and no `.rs` under
  `src/`, `tests/`, `examples/`, `benches/`, `crates/` opens it (the only Rust hit for `VERSION` is
  `src/cli/commands/history.rs:75`, a table column header). It exists for the shell release scripts,
  which run from a git checkout.
- **Publish set is 9 crates.io artifacts, in three steps, not two** — I had downgraded this to 8 after
  reading only `release.yml:352`'s `CRATES="csm-chaos csm-core-lib csm-traits csm-embedding csm-memory
  csm-retrieval csm-persistence"`. Re-reading the whole job:

  | Step | Line | Artifacts |
  |---|---|---|
  | `Publish workspace crates incrementally` | `:344` | 7 (the `CRATES` loop) |
  | `Publish root crate to crates.io` | `:379` | 1 (`chaotic_semantic_memory`, `--locked`) |
  | `Publish csm-duckdb` | `:392` | 1 (`csm-duckdb`) |

  9 crates.io + 2 npm (`@d-o-hub/chaotic_semantic_memory`, `@d-o-hub/csm`) = **11 registry artifacts**,
  plus the GitHub release. `csm-duckdb` sits outside the loop because it dev-depends on the root:
  `:392` polls the sparse index for root `0.3.9` up to 30×15 s before publishing it. That ordering was
  already recorded (`plans/GOAP_STATE.md` `csm_duckdb_in_release_publish_order: true`, 2026-09-30, and
  `plans/ACTIONS.md`) — my "8" was a re-derivation that lost a recorded fact, and the recorded fact was
  right. `csm-wasm` and `csm-cli` are deliberately not on crates.io: no crate depends on them by
  version, they ship over npm.
- Each publish step is idempotent — it probes `index.crates.io/<splat>/<name>` for `"vers":"0.3.9"` and
  skips, so a re-run after a partial failure does not double-publish. Splat paths are 2/2/4/… segments
  (`ch/ao/chaotic_semantic_memory`, `cs/m-/csm-duckdb`); an earlier probe of mine used
  `ch/aotic_semantic_memory`, got HTML back, and I misread that as "no versions published".
- Live index state on 2026-10-30: **all 9 crates sit at 0.3.8**, so every channel has a real 0.3.9 to
  write.
- Caveat for local rehearsal: `cargo package` **cannot resolve offline** at `0.3.9`
  (`failed to select a version for the requirement csm-chaos = "^0.3.9" ... candidate: 0.3.8`) because
  path deps become version requirements and the cached index predates the bump. Local `--offline` dry
  runs are useless for an unpublished version; CI publishes in dependency order with network.

## 3. npm channels — one claim reversed, and what actually survives

- `@d-o-hub/chaotic_semantic_memory` dist-tag latest **0.3.8**; `@d-o-hub/csm` latest **0.3.8**.
- `npm publish --dry-run` in `cli-npm/` succeeds: `@d-o-hub/csm@0.3.9`, 6 files, 4.8 kB; `postinstall.js`
  is the payload and no binary is embedded, as expected for a launcher package.
- **Reversed**: my working notes asserted "provenance is structurally impossible as written" and that
  **0 of 11** published npm versions carry an attestation, from a registry probe. The probe read
  `versions[*].attestations`; the field is `versions[*].dist.attestations`. One segment short, so every
  version returned empty and the instrument could *only* print zero. Measured at the same endpoint with
  the right path: **19 of 21 versions are attested** — `@d-o-hub/chaotic_semantic_memory` 15/16 (only
  0.1.0 is not), `@d-o-hub/csm` 4/5 (only 0.3.4 is not), both `latest` tags included.
- **What survives the correction**, and it is smaller: (a) `verify-release` (`:621`) never reads
  `.dist.attestations` at all — it checks `npm view … version` — so nothing verifies provenance after
  publishing; and (b) the `npm publish --provenance … || npm publish …` fallback at `:548`/`:617` buries
  the first error, though it is harmless today because `publishConfig.provenance = true` in both
  manifests applies to the retry too. The proposed "accept token publishing, drop `--provenance`" would
  have **removed attestation that exists**. Filed and rescoped as #955.

## 4. GitHub-side gates — clear

- Environments `crates.io`, `npm`, `npm-cli`: `protection_rules: []`, `can_admins_bypass: true`, no
  branch policy → **no wait timer and no manual approval** can stall the publish.
- Secrets: repo has `CARGO_REGISTRY_TOKEN` + `NPM_TOKEN`; env `crates.io` carries its own
  `CARGO_REGISTRY_TOKEN`. `npm`/`npm-cli` have no env-scoped secrets, so they fall back to the
  repo-level `NPM_TOKEN`.
- Tag state: `git ls-remote --tags origin 'refs/tags/v0.3.9*'` → empty; latest release is `v0.3.8`.
  So `Ensure release tag exists` will set `release-needed=true`.
- `scripts/pre-release-gate-summary.sh` and `check-audit-ignore-parity.sh` → rc 0 ("RUSTSEC ignore sets
  match (13 ids)").

## 5. The two blockers, and their state

**#953 (the `workflow_run` race) — RESOLVED 2026-10-30T14:35Z, merged as `4ac8422d` (PR #962).**
`release.yml` and `pre-release-gate.yml` both fire on the same CI completion, and `validate` reached its
gate read inside ~62 s while the gate run was still open (measured on run 39648046550), so the one-shot
read saw `in_progress/none` and refused. The merged fix waits for a verdict, bounded:
`GATE_DEADLINE=$(( $(date +%s) + 1800 ))`, `sleep 60`, break on `status == completed`, then still
require `conclusion == success` — fail-closed is preserved, and anything else still blocks. Both arms
were exercised against real runs before merging (the failure mode that made proof mandatory: an
unproven wait turns an instant refusal into a 30-minute one).

**The drain gate — resolved by draining, not by labeling (owner decision).** `pr-drain-check`
(`pre-release-gate.yml:25`) calls `gh pr list --state open` and fails if any PR lacks
`hold-for-next`/`deferred`; `gate-summary` `needs:` it, so the run concludes `failure` and the release
refuses. Proven from run 40708535880 (head `ae117fb9`): six jobs green, `pr-drain-check` red on one open
PR. The owner chose to roast and resolve the queue rather than label other people's PRs: open PRs went
4 → 0, the one genuine defect preserved as issue **#961**. **Issues do not trip this gate**
(`gh pr list`, not `gh issue list`), which is why #961/#964/#965 are filed as issues while the release
is in flight. At the time of writing the only open PR is **#963** (docs), and it is the last thing
standing between `main` and the tag.

**Merge-time check added by this round**: a squash subject is `<PR title> (#NNN)`, so the budget is
`100 − len(" (#NNN)")` and no PR-side gate measures it. #947's title (97) was legal, its subject (104)
made `main` red (`header-max-length`, run 41059071360 / check-run 124073904392), and a red `main`
refuses every dependent `workflow_run`. Measured safe here: #962 80, #963 92. Filed as **#965**.

## 6. Runbook for the dynamic half (only `main` can prove these)

1. Merge #963 (squash) → `main` advances → CI runs on the new head with **zero open PRs**.
2. CI completion fires `pre-release-gate.yml` and `release.yml` for the same SHA; `validate` now waits
   for the gate verdict (≤30 min) instead of reading it once.
3. `release.yml` creates `v0.3.9` from `Cargo.toml`, then publishes 9 crates in dependency order
   (7 → root → `csm-duckdb`), builds and publishes both npm packages, and creates the GitHub release.
4. **Read the `validate` job log**, do not infer from a green tick: confirm the line
   `Pre-Release Gate on <sha>: status=completed conclusion=success` actually printed, otherwise the run
   refused before tag creation and nothing is published.
5. Post-publish verification (the half this file cannot claim):
   - `cargo new && cargo add chaotic_semantic_memory@0.3.9 && cargo build` — proves the index artifact
     resolves, independent of path deps; then `cargo add csm-duckdb@0.3.9` in a throwaway project.
   - `npm view @d-o-hub/chaotic_semantic_memory@0.3.9` and `npm view @d-o-hub/csm@0.3.9`, plus
     `npm audit signatures` for attestation (positive control: 0.3.8 is attested, `csm@0.3.4` is not).
   - Download every `v0.3.9` release asset and run `--version` on at least the linux-x64 CLI binary.
   - `https://docs.rs/crate/chaotic_semantic_memory/latest` builds.
   - Note that `verify-release` (`:621`) reports on only **3 of the 11** artifacts, and its job carries
     no `shell:` override, so under Actions' default `bash -e {0}` the first miss aborts the step and
     its `::error::Partial release detected!` summary is unreachable. A green `verify-release` is
     therefore weak evidence about the other 8 — that gap is #954.
6. If any step after the tag exists fails, re-run with `workflow_dispatch` `recover=true` +
   `recovery_tag=v0.3.9`; never delete the tag.

## 7. Known non-blocking drift carried into 0.3.9

- Three member manifests pin `chaotic_semantic_memory = { path = …, version = "0.3.8" }` under an
  `0.3.9` tree (`crates/csm-cli/Cargo.toml:11`, `crates/csm-wasm/Cargo.toml:11`,
  `crates/csm-duckdb/Cargo.toml:25`) while `verify-version-sync.sh` exits 0 — the script writes and
  checks the root→member edge only, never the reverse. Not a blocker (`^0.3.8` accepts 0.3.9, and only
  `csm-duckdb` of the three reaches crates.io), expensive at the next major. Filed as **#964**.
- Four `zstd` advisories shipped **ignored, not fixed** by #946; disclosed in the CHANGELOG so a
  published release note does not imply a clean advisory picture.
