# ACTIONS — Active GOAP Action Queue

> **Compacted 2026-08-08 (ADR-0097).** This file now holds **active actions
> only** (`queued` | `in_progress` | `blocked` | `deferred`).
> Completed actions (292 entries, 2026-02 → 2026-08) are archived verbatim at
> `plans/.archive/2026-08-08-historical/ACTIONS_2026_08_08.md` (full snapshot,
> includes both completed and active sections as of the compaction date).
> Earlier history: `plans/.archive/2026-07-20-historical/` and git history.
>
> Hygiene: when an action completes, remove it here, set
> `action_last_completed` in `GOAP_STATE.md` (exactly once), and let the next
> dated reconciliation snapshot the file again. Do not re-add completed
> entries to this file.
>
> Last completed (verified 2026-09-17):
> `clear_dependabot_queue_2026_09_17` (no ADR; supply-chain hygiene) — merged
> the WASM bincode fix (#725) plus the whole Dependabot backlog: consolidated
> batch PR #726 (`fuzz/Cargo.lock`, `taiki-e/install-action` 2.87.12,
> dependabot double-scope fix) superseding #717–#722, native #727
> (`clap_complete`), and two real migrations instead of ignores — rmcp
> 2.2 → 3.4 (#728: MRTR response enums, `with_all_items`, `ServerConfig`) and
> opentelemetry 0.27 → 0.32 (#729: clears GHSA-w9wp-h8wv-79jx, medium;
> `SdkTracerProvider`, `Resource::builder`). Root cause of the recurring
> `commitlint` failures: `prefix: "chore(deps)"` + `include: "scope"` emits
> `chore(deps)(deps):`, and the PR-title guard keyed on `github.actor` stopped
> skipping the bot after a maintainer ran `update-branch`. Zero open PRs and
> zero open issues after #730; 3 alerts remain non-actionable
> (`lru` GHSA-rhfx-m35p-ff5j pinned at `^0.12` by quinn-proto/tantivy,
> `libsql-sqlite3-parser` ×2 with no patched release). The three P2/P3
> actions below stay queued; `action_last_completed` unchanged because no
> queued GOAP action was in scope.
>
> Last completed (verified 2026-09-15, wave 2):
> `deduplicate_persistence_owner_bodies` (ADR-0094) — phase 1 (crate parity) was
> already on main; this wave promoted the bridge types to `csm-traits`
> (`ConceptGraph`, PR #711), moved the canonical graph CRUD into
> `csm-persistence` behind a delegating root facade (PR #711), converged the
> export payloads onto the owner-neutral `csm-traits` types (PR #713) and
> dropped the last dead copy in `csm-persistence` (this PR). Root keeps no
> second body of persistence or export payloads — only re-exports, one-line
> delegations and infallible converters; the payload schema is unchanged
> (field-level decode of pre/post bincode payloads identical except the
> wall-clock field and pre-existing map ordering; cross-imports work both
> ways). Partial progress on
> `deduplicate_test_and_source_surfaces` (P3) — action stays queued for the
> remaining root/crate test-body duplicates.
>
> Last completed (verified 2026-09-15):
> `triage_pr_roast_2026_09_15` — 0 open issues, 2 open PRs. Jules draft
> #706 (`perf(retrieval)` hybrid merge pass) closed as no-impact after
> allocator-level measurement (identical outputs, identical allocation counts
> and bytes, timing deltas opposite in sign); dependabot #707
> (`taiki-e/install-action` 2.87.10 → 2.87.11 SHA pin) verified green and kept
> for manual merge. Record: `plans/PR_ROAST_2026_09_15.md`. Same session queued
> the 0.3.8 version-truth repair, artifact hygiene, the `probe_with_rerankers`
> restoration, and the user-approved ADR-0094 persistence dedup phases.
>
> Last completed (verified 2026-09-13, wave 3):
> `dedupe_hybrid_root_shim_2026_09_13` — PR #704 merged (`9429a07`): root
> `src/retrieval/hybrid.rs` is now a 9-line re-export shim over
> `csm_retrieval::hybrid` (bm25/rerank pattern), `compute_weights` exported
> from the crate (was unreachable — the dead-copy trap #689 hit, found before
> it re-broke the CLI), root `hybrid_tests.rs` and the orphaned
> `src/retrieval/bm25/tests.rs` from #647 deleted (≈ −550 LOC). Hybrid has one
> compiled implementation; the dual-surface bug class from
> `plans/PR_ROAST_2026_09_12.md` is structurally closed. Partial progress on
> `deduplicate_test_and_source_surfaces` (P3) — action stays queued for the
> remaining root/crate test-body duplicates. mutation-test + miri verified
> green under setup-rust-toolchain v2.0.0.
>
> Last completed (verified 2026-09-13, wave 2):
> `repair_setup_rust_toolchain_2_0_0_warnings_cascade_2026_09_13` — merged the
> weekly dependabot batch (#694–#698, incl. `setup-rust-toolchain` 2.0.0) and
> repaired its fallout: v2.0.0 moved deny-warnings from `RUSTFLAGS` to cargo
> config (`CARGO_BUILD_WARNINGS=deny`), which `RUSTFLAGS`-overriding builds
> (cargo-fuzz `--cfg fuzzing`, `cargo miri setup` sysroot) cannot escape —
> fixed with `CARGO_BUILD_WARNINGS=allow` on those jobs (PRs #701/#702; valid
> levels are warn/allow/deny, "none" is rejected, key respected only by cargo
> ≥ 1.97). PR #700 merged: static `ENV_TEST_LOCK` serializes env mutation in
> embedding provider tests (intermittent voyage-test failure, run
> 103702134133). Fuzz re-dispatched on main to re-validate the nightly path.
>
> Last completed (verified 2026-09-13):
> `align_hybrid_dual_surface_selection_2026_09_13` — PR #693 merged
> (`cda3c26`): 0-based `top_k - 1` selection + `let nth` sibling style applied
> to BOTH hybrid copies (root `src/retrieval/hybrid.rs` — the copy the CLI
> runs — and `crates/csm-retrieval/src/hybrid.rs`), truthful boundary-test
> rationale in both test copies, `len == top_k + 1` mutation-kill tests, and
> documented excludes for the two equivalent mutants. PR #689 closed as
> superseded. PR #692 merged: dependabot cargo ignore for upstream-blocked
> `libsql-sqlite3-parser` (GHSA-8m95-fffc-h4c5, no patched release; weekly
> security job stopped erroring). First scheduled fuzz-full green since
> 08-02 (run 34746887403).
>
> Last completed (verified 2026-09-12):
> `fix_chronic_main_ci_failures_2026_09_12` — PR #690 merged (`0e569b7`):
> fuzz jobs now run nightly via ci.yml's pinned setup action (nix shell
> dropped; the scheduled full run had been red six consecutive Sundays
> 08-02 → 09-06; workflow_dispatch validation 34709772141 green) and Release
> `wait-for-ci` ceiling raised 1800s → 2700s / job timeout 55m (8 timeouts
> with CI green, 09-07 → 09-09). PR #689 roasted — fixes the dead
> `crates/csm-retrieval` copy of `merge_results` with no bench evidence and a
> stale boundary-test rationale; review comment posted, changes requested.
> Record: `plans/PR_ROAST_2026_09_12.md`.
>
> Last completed (verified 2026-09-07):
> `resolve_open_github_issues_and_pr_triage` — Closed superseded duplicate
> PR #672; verified and squashed PR #647 implementing all 4 open GitHub issues
> (#639, #640, #641, #642); deduplicated root retrieval modules to re-export shims
> (-5,300+ total repo LOC); updated PR titles (#670, #660); verified Codacy pass on all PRs.
>
> Last completed (verified 2026-08-12):
> `reconcile_pr_wave_2026_08_12` — PR roast wave landed
> (#620/#621/#622), BM25 absence wired, wave-33 flags trued,
> csm-cli/csm-wasm dead dupes deleted, bench harnesses shipped.
>
> Last completed (verified 2026-08-11):
> `replace_persistence_disabled_noops` (ADR-0094) — persistence-disabled
> configuration can no longer return false success: `with_local_db`/`with_turso`
> in no-persistence builds record config and `build()` rejects with
> `UnsupportedOperation`; the no-persistence `persistence` module methods all
> return `UnsupportedOperation`; un-gated persistence/CLI integration tests and
> examples were feature-gated so the full `--no-default-features` matrix compiles
> and passes. `--no-default-features --features cli` keeps compiling (ADR-0067).
>
> Last completed (verified 2026-08-08):
> `enforce_workspace_feature_contracts` (ADR-0094) — owner deps
> `default-features = false`, `persistence`/`parallel` forward explicitly,
> rayon optional in csm-memory, workspace MSRV 1.88 single-sourced;
> `cargo tree --no-default-features` has no libsql/rayon.
> Earlier same-day completions (ADR-0097): `harden_public_f32_api_validation`
> (PR #607), `recover_v037_failed_deployments` (v0.3.7 + v0.3.8 on crates.io).

> Last completed (verified 2026-09-23):
> `enforce_perf_claim_evidence_gate` — fail-closed CI check for the
> #737/#739/#740/#754 class: `scripts/check-perf-pr-evidence.py` runs in the
> always-on `commitlint` job of `ci.yml` for every PR and, for `perf(...)`
> titles, requires a `## Performance Evidence` section (new PR template
> section) naming a `plans/evidence/bench/canonical.json` benchmark id (or the
> nearest canonical comparator when none exists), a path/URL-shaped Criterion
> output or flamegraph reference (a bare `flamegraph` word does not count), and
> a numeric before/after pair; title/body arrive as argv with environment
> fallback, scope validity stays with commitlint. Nineteen-case local matrix
> exercised (non-perf/docs/bot/empty titles pass; missing section, baseline,
> artifact, numbers, placeholder-only, untouched template and "template plus
> numbers only" bodies fail; complete Criterion/flamegraph and argv
> invocations pass). Syntactic presence/shape only — it does not validate
> provenance or assert improvement. Prerequisite `fix(core)` lint repair (#759,
> merged `e5d039e`) unblocked the workspace-wide clippy sensor of the mandatory
> local harness gate, and the `lint` CI job gained the test job's RAM-backed
> `/tmp` mount (same persistence-latency gate, previously red with zero diff).
> `validated: false` and `reconcile_wave_32_remainder_and_flag_truth` stand:
> the July audit has other exit criteria.

> Last completed (verified 2026-09-18):
> `triage_pr_roast_2026_09_18` — two open PRs. #735 (`fix(framework)`: GraphRAG
> config validation) verified against the owner crate's `MAX_TRAVERSAL_DEPTH`,
> all entry points and the existing tests; two non-blocking nits; keeper, merge
> after `update-branch`. #737 (`perf(memory)`: LSH hashing + candidate scoring)
> closed as a no-op — the branch changed zero files, and its two ideas were
> adjudicated with allocator counts and the existing ANN evidence so a
> resubmission has a target to beat. Record: `plans/PR_ROAST_2026_09_18.md`.
>
> Last completed (verified 2026-09-17, evidence wave):
> `add_ann_and_persistence_scale_benchmarks` + `replace_formula_only_memory_claim`
> (ADR-0095) — the scale-evidence runner (PR #733) measures exact/HNSW/LSH/
> bucketed retrieval at 1 k/10 k/50 k, persistence throughput and contention, and
> a bytes-per-concept model with a held-out point; artifacts and manifests live
> in `plans/evidence/scale_2026_09_17/`. The persistence contention half was red
> before the fix (0.905 error rate with eight writers) and green after PR #734
> added a 5 s busy timeout plus bounded retries to `csm-persistence`
> (0.000 error rate, same workload, before/after artifacts side by side). The
> memory claim is now measured, not asserted: 4 691 B RSS and 2 850 B persisted
> per concept (held-out error 0.29 % / 0.06 % at 100 k), so the recorded
> `< 12 MB for 10M concepts` target — which described the unimplemented ADR-0024
> phase-2 product quantization — is marked not supported in the book and
> `docs/architecture/context.yaml`, and the formula-only test was replaced by a
> measured one. `deduplicate_test_and_source_surfaces` (P3) stays queued.

> Last completed (verified 2026-09-20):
> `triage_pr_roast_2026_09_20` — two Jules perf PRs (#739, #740) against the same
> `merge_single_list` hunk in `crates/csm-retrieval/src/hybrid.rs`, both replacing
> a `TrustedLen` `collect()` with `with_capacity` + push: allocator counting showed
> byte-identical allocation profiles (102/6 090 B at n=100, 1 002/61 890 B at
> n=1000, both variants), so both were roasted and closed as no-ops; #740's
> 13.67 % claim had no criterion output. #735 from the 2026-09-18 wave stays
> merge-ready. Record: `plans/PR_ROAST_2026_09_20.md`.
>
> Last completed (verified 2026-09-18, test-surface dedup):
> `deduplicate_test_and_source_surfaces` (ADR-0094, ADR-0095) — removed the 24
> facade test bodies that were byte-identical duplicates of owner-crate tests
> (12 in `src/embedding/mod.rs`, 6 in `src/persistence_wasm.rs`, 3 with the
> `csm-core-lib` neural-circuit copy, plus three singles), turned
> `csm-core-lib::maps::neural_circuit` into a re-export of the `csm-chaos`
> owner (its feature was enabled by nobody, so the second body was
> unreachable), and ported the `ENV_TEST_LOCK` race fix into `csm-embedding`
> before deleting the facade copies that held it. Six further "duplicate" files
> were *rejected* by mechanical body comparison and hand review — they assert
> distinct behavior (metrics vs membership, error variants, retrieval after
> TTL) and stay. Coverage methodology replaced: `scripts/update-coverage.sh`
> (test-LOC ratio, `crates/` blind) → `scripts/coverage-report.sh`
> (unique compiled behavior; llvm-cov line/branch: 74.33 % lines / 64.08 %
> branches over lib + integration targets). Audit: `plans/TEST_SURFACE_AUDIT_2026_09_18.md`.

> Last completed (verified 2026-09-21):
> `fix_bucketed_candidate_recall_at_scale` (ADR-0095 evidence → ADR-0065 path) —
> the bucketed candidate generator no longer loses recall as the corpus grows:
> the probe width scales with N (`effective_bucket_probe_width`, configured
> width as a floor, capped at `MAX_BUCKET_PROBE_WIDTH`), candidates within one
> masked bit of the query bucket are accepted (10 k: recall@10 0.604 → 0.832 at
> floor 8, 249 µs vs 558 µs exact), and an oversized probe now returns nothing
> so the caller scans exactly instead of slicing the bucket by index order — the
> failure that measured 0.016 recall at floor 2. Evidence and the sweep table:
> `plans/evidence/scale_release_2026_09_21/` (`bucket_sweep/` holds pre- and
> post-fix runs at 10 k/50 k/200 k for floors 2 and 8).

> Last completed (verified 2026-09-22):
> `triage_pr_roast_2026_09_22` — one open PR. #754 (`perf(memory)`: pre-allocate
> capacity + drop iterator closures in `singularity_retrieval`) closed as no
> demonstrated impact: counting-allocator adjudication showed hunk 4 (both scan
> tails) byte-identical (257 allocs / 11 264 B — `TrustedLen` `collect()` is
> already single-exact-alloc), hunk 2 self-contradicting (`with_capacity(len.min(max))`
> then pushing `len` before `truncate`), and the remaining ~13-alloc growth-chain
> delta dwarfed by the 257 `String` clones the rewrite keeps. No criterion
> evidence attached (4th entry in the #737/#739/#740 class). Record:
> `plans/PR_ROAST_2026_09_22.md`. Five followups queued below; `action_last_completed`
> unchanged because no queued GOAP action was in scope.

> Last completed (verified 2026-09-30):
> `triage_pr_roast_2026_09_30` — drained the open queue: #791 (owner-crate cache
> invalidation + capacity-safe upsert) merged as `cd70fa5`, #789 (2026-09-28 roast
> record + skill lesson) as `977f36f`, and #792 (2026-09-29 recommendations +
> `plans/PR_ROAST_2026_09_30.md`) as `195d6e61`; #790/#793 closed with roast
> comments for selecting the wrong candidate under negative public weights, and
> #783 closed earlier as zero-delta. Every keeper was rebased onto the current
> `main`, commitlint-validated, and re-checked on the exact new head before an
> explicit squash merge — never `gh pr merge --auto`. Records:
> `plans/PR_ROAST_2026_09_28.md`, `plans/PR_ROAST_2026_09_29.md`,
> `plans/PR_ROAST_2026_09_30.md`. 0 open PRs, 0 open issues; the three queued
> actions below are unchanged (none was in scope).

> Last completed (verified 2026-09-30, wave-32 exit pass):
> `reconcile_wave_32_remainder_and_flag_truth` — re-verified every exit
> criterion of `plans/GOAP_AUDIT_2026_07_14.md` against HEAD `7ce6fb73` instead
> of trusting recorded flags. Met: ANN config fallible + revisioned,
> fingerprint-gated snapshots (`tests/ann_revision_envelope.rs`); fuzz
> workspace compiles (exact CI command, exit 0); lean `--no-default-features`
> graph (0 libsql/rayon on normal+build edges); one WASM artifact (CI + release
> both `build-wasm.sh release-web`); full workspace/supply-chain CI; bulk
> association load; benchmark metric math + hand-calculated tests; measured
> memory claim; absence short-circuit wired; 0 TODO/`unimplemented!`; unique
> source/test ownership; compact plans. Remainder queued as nine actions (TTL
> shutdown ownership, absence invalidation, failure-path test, query-count
> test, gate/fixture/hook consolidation, skill catalog + agent-context
> generation, scheduled/release evidence tiers + mutation inventory,
> machine-derived CI matrix, archive-manifest validation). `validated` stays
> false with that evidence-based justification; `wave_32_status` stays
> in_progress.

> Last completed (verified 2026-09-30, release gate):
> `migrate_release_wait_for_ci_to_workflow_run` — PR #796 merged as
> `8c1f864d`: `release.yml` triggers on the CI workflow completing on `main`
> (`workflow_run`) instead of a push plus a 45-minute poll, releases exactly
> `github.event.workflow_run.head_sha`, gates the job on
> `conclusion == 'success'` + same-repo + `event == 'push'`, and keeps
> `workflow_dispatch` fail-closed with a single CI query. Proof: trigger run
> 36746741481 (`validate` success, every publish job skipped, and no
> push-triggered release run for the merge SHA) plus dispatch run 36746882951
> (log `CI on 8c1f864d…: status=completed conclusion=success`, then
> `Tag v0.3.8 already exists; skipping release.`).

> Last completed (verified 2026-09-30, publish pre-check):
> `fix_crates_publish_precheck_and_add_duckdb` — PR #798 merged as
> `71a48618`: the name pre-check is ownership-based (owners API; free or
> `d-o-hub` passes, other owners fail, unknown HTTP fails closed), version
> existence comes from the sparse index (fail-closed if unreadable), the
> companion loop classifies failures instead of swallowing them, and
> `csm-duckdb` has a dedicated step after the root publish that waits for the
> root version in the index. Verified by executing the workflow's extracted
> shell: eight-name list green, `serde` conflict red, `0.3.8`/`0.3.9`
> published-probe split correct, duckdb step no-ops on the published version.

> Last completed (verified 2026-10-01, TTL lifecycle):
> `own_ttl_cleanup_shutdown` — PR #801 merged as `96144703` (ADR-0099):
> cooperative `watch` cancellation replaces `Drop`-time `abort()`, the loop
> also stops when the last handle drops, `shutdown()` cancels and awaits under
> a 5 s bound (idempotent, shared across clones, no-op on wasm32), and two
> CI iterations were root-caused on the way — the 500-line LOC gate via
> child-module extraction (`src/framework_cleanup.rs`) and three missed
> mutation mutants via in-crate unit tests, not exclusions. Verification:
> 20 TTL integration tests + 2 unit tests + 6 arch-fitness tests pass locally,
> and CI on `49b6a74` has lint, test, mutation-test, miri, workspace crates,
> benchmark-small and commitlint green.

> Last completed (verified 2026-10-01, triage):
> `triage_pr_roast_2026_10_01` — one open PR. #800 (`perf(core)`: zero-shift
> fast path for `HVec10240::permute`, draft) kept open with an evidence
> request: correct (byte-identical for `shift ≡ 0 (mod 10240)`, locked by
> `hyperdim_tests.rs:48`) and reachable (`encoder.rs:226` calls `permute(0)`
> for position 0), but `commitlint` is red on the perf gate (`missing
> '## Performance Evidence' section in the PR body`) and the only `permute`
> bench uses shift `321`, which cannot enter the new branch. Record:
> `plans/PR_ROAST_2026_10_01.md`; lesson distilled into
> `.agents/skills/pr-roast-triage/SKILL.md`. No queued GOAP action was in
> scope; counters unchanged.

> Last completed (verified 2026-10-01, absence invalidation):
> `add_absence_invalidation_semantics` — PR #804 merged as `c8f91876`: absence
> rows now carry namespace + namespace_revision, so any durable mutation makes
> them stale (no write on the mutation path), successful probes delete the
> record they found, and the attempt counter restarts when the revision moved.
> Migration v12 adds the columns with ('', 0) defaults that keep old rows inert.
> Verified by a new regression (fails with `got ["AbsenceShortCircuit"]` when the
> revision comparison is removed), a parked-v11 upgrade check, four root-caused
> CI iterations ending at a 100% mutation score, and a green CI on `abff706`
> (lint, test, mutation-test, miri, nine workspace crates, deny, commitlint).

actions:
  - name: add_persistence_failure_path_test
    preconditions: []
    effects:
      persistence_failure_semantics_tested: true
    notes: >
      Audit C3, re-verified 2026-09-30. The behavior exists — persist before
      mutate plus reload-reconcile (`src/framework_persistence.rs:244-296`) —
      but no test injects a persistence error: grep finds no failing/mock
      `Persistence` and no `inject_concept(...).unwrap_err()` assertion in
      `tests/`. Required: a failure-injection test asserting
      `stats().concept_count` is unchanged after a failed inject/delete, so
      `persistence_failure_leaves_memory_unchanged` rests on a gate.

  - name: wire_and_consolidate_validation_gates
    preconditions: []
    effects:
      single_gate_graph: true
    notes: >
      Audit W1/G3, re-verified 2026-09-30. Negative fixtures exist
      (`scripts/test-llms-sync.sh` 5 cases, `test-version-sync.sh` 4 cases,
      `negative-fixtures.sh`) but no gate invokes them;
      `validate-workflows.sh`, `validate-git-hooks.sh` and `validate-links.sh`
      have zero callers; three hook installers (`install-hooks.sh`,
      `setup-hooks.sh`, `validate-git-hooks.sh --install`) install different
      hook sets; `pre-commit.sh` and `hooks/pre-push` enforce different sensors
      than `harness-check.sh`; the CI-wired skill validator has no negative
      fixture. Required: one bootstrap, one canonical gate graph, fixture tests
      wired into `validate.sh` + CI (or deleted), negative fixture for the
      skill validator.

  - name: generate_skill_catalog_and_agent_context
    preconditions: []
    effects:
      skill_catalog_generated_and_gated: true
    notes: >
      Audit Phase-4 exit, re-verified 2026-09-30. `.agents/skills/CATALOG.md`
      says "32 skills" while the disk has 33, with no generator and no drift
      check in `scripts/`, `.github/` or hooks. Same family:
      `scripts/gen-agents-context.sh:77` hardcodes "Skills (13 Total)" and
      `:13` reads archived `plans/SWARM_COORDINATION.md`;
      `docs/architecture/context.yaml` is stale (`current_status.active_wave:
      11`, `total_tests: 134`, `skills.total_count: 19`, `adrs.count: 16`) and
      its `architecture.modules` names pre-extraction root files. Required:
      generator + drift gate for the catalog, and refreshed (or generated)
      agent-context artifacts with a checker.

  - name: complete_evidence_tiers_and_mutation_hardening
    preconditions: []
    effects:
      scheduled_and_release_evidence_tiers: true
      mutation_inventory_published: true
    notes: >
      Audit E4/E6 + ADR-0095 tiers, re-verified 2026-09-30. The PR tier exists
      (`benchmark-ci.yml`; `ci.yml` test-benchmarks, graph-candidates,
      perf-evidence gate), but no scheduled workflow runs the scale evidence
      (`scripts/scale-evidence.sh`) or a full mutation sweep;
      `pre-release-gate.yml` (workflow_call + workflow_dispatch) has no caller
      and runs no benchmark; `benchmark-ci.yml` path filters omit `crates/**`,
      so owner-crate changes skip it. Mutation: timeouts are unresolved ✓ and a
      budget fails the job ✓, but the static exclude list
      (`scripts/mutation_test.sh:181-262`) still applies to changed files and
      only aggregate counts are printed (no module-level inventory artifact).

  - name: add_query_count_regression_test
    preconditions: []
    effects:
      bulk_association_load_verified: true
    notes: >
      Audit P1 remainder, re-verified 2026-09-30. `load_all_associations`
      (`crates/csm-persistence/src/persistence_index.rs:148`) is used by
      `load`/`load_replace`/`load_merge` (`src/framework_persistence.rs:100,184,304`),
      so the N+1 loop is gone, but nothing counts queries — grep for
      `query_count|num_queries` finds no hits. Required: a query-count
      regression test pinning the single namespace-scoped association load.

  - name: derive_ci_crate_matrix_from_workspace
    preconditions: []
    effects:
      ci_matrix_machine_derived: true
    notes: >
      Audit A6 remainder, re-verified 2026-09-30. `ci.yml`'s
      `test-workspace-crates` matrix (`ci.yml:202-241`) is a hand-written crate
      list guarded only by a "Keep in sync with workspace members" comment, so
      a new `crates/*` member can silently miss CI. Required: derive the matrix
      from `cargo metadata`, or add a check that fails when the list and the
      workspace diverge.

  - name: validate_archive_manifest_completeness
    preconditions: []
    effects:
      archive_manifest_validated: true
    notes: >
      Audit G4, re-verified 2026-09-30. `plans/ARCHIVE_MANIFEST.md` documents
      the 2026-07-20/2026-08-08 compactions and `plans/README.md` links it, but
      no script reads it (grep for ARCHIVE_MANIFEST across `scripts/` and
      `.github/` is empty) and it does not enumerate the 55 top-level archived
      ADRs under `plans/.archive/` (`grep -c 'plans/.archive/00'` = 0).
      Required: list the archived ADRs and add a checker so
      `plan_archive_manifest_valid` rests on a gate.

  - name: wire_graceful_shutdown_into_servers
    preconditions: []
    effects:
      servers_stop_ttl_cleanup_explicitly: true
    notes: >
      ADR-0099 added `ChaoticSemanticFramework::shutdown()` (bounded await),
      but no caller uses it: `src/mcp/server.rs` and
      `src/cli/commands/watch.rs` rely on dropping the framework, which now
      stops the cleanup task cooperatively at its next check. Required: call
      `shutdown().await` on those exit paths so a long-running process stops
      the task deterministically and can surface a stuck task; cover the
      server path with a test.

  - name: trigger_wasm_job_on_root_src_changes
    preconditions: []
    effects:
      wasm_job_runs_for_root_src_changes: true
    notes: >
      `ci.yml`'s `detect-changes` sets `wasm=true` only for
      `^(crates/csm-wasm/|src/wasm|wasm/|Cargo\.(toml|lock)$)`, so a change to
      any other root `src/**` file — including `cfg(not(wasm32))` gating that
      the wasm build must compile — skips the `wasm` job entirely (observed on
      PR #801, where all wasm validation was local). Required: include the root
      crate's `src/**` (or the crate the wasm package depends on) in the
      filter, and assert the job runs for such a diff.

