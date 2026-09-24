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

actions:
  - name: publish_csm_duckdb_companion
    preconditions:
      ci_all_checks_passed: true
    effects:
      duckdb_companion_published: true
    notes: >
      csm-duckdb is the last unpublished companion crate (duckdb_companion_published
      flag). Follow agents-docs/release-safety.md: trusted publishing, synchronized
      Cargo.lock, verify ownership before `cargo publish`.

  - name: eliminate_retrieval_string_clones
    preconditions:
      perf_pr_evidence_gate_enforced: true
    effects:
      retrieval_string_clones_removed: true
    notes: >
      The real hot-path win identified by the #754 roast: `score_specific_candidates`
      and the scan tails materialise `(String, f32)` with a clone per candidate
      (257 allocs / 11 KB per query at n=256). Carry `usize` indices to the API
      boundary or return borrowed/Arc<str> ids. Must beat the criterion baselines
      in plans/evidence/scale_release_2026_09_21 with attached numbers.

  - name: reconcile_wave_32_remainder_and_flag_truth
    preconditions: []
    effects:
      validated: true
    notes: >
      GOAP_STATE flag-truth pass: wave-32 "ownership + evidence remain" comment
      is stale (ownership dedup landed 2026-09-15/18, evidence waves 2026-09-17/21),
      and `validated: false`'s justification with it. Verify against
      plans/GOAP_AUDIT_2026_07_14.md exit criteria, then flip in place.

  - name: regenerate_stale_llms_dependency_versions
    preconditions: []
    effects:
      llms_dependency_versions_current: true
    notes: >
      Committed `llms.txt`/`llms-full.txt` drift from `Cargo.lock`: they still
      list opentelemetry 0.27 while the lockfile resolved 0.32.0 after the
      #729 security bump. Last regenerated at #714, before that bump.
      `scripts/validate.sh` regenerates and validates these files but never
      commits them, so the drift accumulates silently. Regenerate, commit, and
      decide whether the regeneration step belongs in a gate that fails on
      drift (CI `lint` runs validate.sh) rather than only warning.

  - name: migrate_release_wait_for_ci_to_workflow_run
    preconditions: []
    effects:
      release_wait_event_driven: true
    notes: >
      `release.yml`'s `wait-for-ci` polls `gh run list` on a `MAX_WAIT` ceiling
      (1800 → 2700 → timed out again on run 36031855839 while main CI on the same
      SHA sat `queued` ~40 min on a saturated runner pool). Polling cannot
      distinguish "CI is slow" from "CI has not been scheduled", and
      `gh run rerun` cannot help a run that is merely queued. Third occurrence
      ⇒ stop raising the ceiling. Migrate the gate to a `workflow_run`
      trigger on ci.yml completion (event-driven, no ceiling), keep the
      `release-needed` version check, and verify with `gh workflow run` plus a
      no-op dry run before trusting it on a real release.

