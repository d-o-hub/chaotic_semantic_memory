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

actions:
  # P1 — ownership and contracts (ADR-0094)
  - name: deduplicate_persistence_owner_bodies
    preconditions:
      adr_0094_accepted: true
      retrieval_implementation_owner_unique: true
    effects:
      workspace_implementation_owners_unique: true
      duplicate_implementation_bodies: 0
    cost: 10
    status: queued
    file: src/persistence*, src/cli/, src/wasm*, crates/csm-persistence/, crates/csm-cli/, crates/csm-wasm/, crates/csm-traits/
    adr: ADR-0094
    description: |
      CLI and WASM duplicate bodies are removed (csm-cli #626, csm-wasm #627);
      persistence/export-payload body convergence between root and
      csm-persistence remains. Migrate with API snapshots and behavior parity; do
      not blindly re-export currently divergent implementations.

  # P2 — evidence (ADR-0095)
  - name: add_ann_and_persistence_scale_benchmarks
    preconditions:
      performance_claims_have_current_artifacts: true
      ann_snapshot_revision_validated: true
    effects:
      ann_scale_evidence_current: true
      persistence_contention_evidence_current: true
    cost: 8
    status: queued
    file: benches/benchmark.rs, benches/persistence_benchmark.rs, benchmarks/
    adr: ADR-0095
    description: |
      Harnesses landed 2026-08-12 (LSH parity bench, persistence CRUD p50/p95/p99
      percentile bench, persisted-bytes metric); full-scale artifacts still
      pending. Compare exact/bucket/HNSW/LSH build, query, update, delete, bytes, recall,
      and reload at agreed scales. Bound persistence retries/timeouts and report
      throughput, p50/p95/p99, retry, and error rates.

  - name: replace_formula_only_memory_claim
    preconditions:
      performance_claims_have_current_artifacts: true
    effects:
      measured_memory_model_exists: true
      ten_million_memory_claim_evaluated: true
    cost: 4
    status: queued
    file: tests/performance_targets.rs, benchmarks/, plans/handoffs/
    adr: ADR-0095
    description: |
      Persisted-bytes metric landed 2026-08-12; full-scale memory model still
      pending. Measure allocator/RSS and persisted/index bytes at multiple scales, fit a
      bytes-per-concept model with held-out error <=5%, then evaluate whether a 10M
      projection is supportable. This action records evidence/evaluation only;
      set support true separately iff the measured acceptance threshold passes.

  # P3 — consolidation (ADR-0094, ADR-0095)
  - name: deduplicate_test_and_source_surfaces
    preconditions:
      workspace_implementation_owners_unique: true
      adr_0095_accepted: true
    effects:
      canonical_test_owners_unique: true
      coverage_methodology_behavior_based: true
    cost: 8
    status: queued
    file: src/, crates/, tests/
    adr: ADR-0094, ADR-0095
    description: |
      Remove duplicated root/split test bodies after owner migration. Report unique
      compiled behavior and line/branch coverage; raw test count remains inventory only.
