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
> Last completed (verified 2026-08-08, ADR-0097):
> `harden_public_f32_api_validation` (PR #607 merged 2026-08-07; validators in
> `crates/csm-memory/src/singularity_decay.rs` + `src/wasm_ext.rs`) and
> `recover_v037_failed_deployments` (recover mode in release.yml; wasm-opt
> `--enable-nontrapping-float-to-int` enabled; v0.3.7 + v0.3.8 both live on
> crates.io).

actions:
  # P1 — ownership and contracts (ADR-0094)
  - name: enforce_workspace_feature_contracts
    preconditions:
      adr_0094_accepted: true
    effects:
      no_default_features_is_lean: true
      msrv_workspace_aligned: true
    cost: 5
    status: queued
    file: Cargo.toml, crates/*/Cargo.toml
    adr: ADR-0094
    description: |
      Disable owner-crate defaults and explicitly forward persistence, parallel,
      ANN, embedding, and protocol features. Acceptance: no-default cargo tree has
      no libSQL/Rayon and every workspace manifest uses the canonical MSRV.

  - name: replace_persistence_disabled_noops
    preconditions:
      adr_0094_accepted: true
      no_default_features_is_lean: true
    effects:
      persistence_disabled_false_success_removed: true
    cost: 3
    status: queued
    file: src/framework_builder.rs, src/lib.rs
    adr: ADR-0094
    description: |
      Remove silently ignored DB builder configuration and Ok/empty persistence
      stubs. APIs are cfg-absent or return UnsupportedOperation consistently.

  - name: consolidate_persistence_cli_wasm_ownership
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
      Complete the owner/facade migration for persistence/export payloads, CLI,
      and WASM. Migrate one concern per PR with API snapshots and behavior parity;
      do not blindly re-export currently divergent implementations.

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
      Compare exact/bucket/HNSW/LSH build, query, update, delete, bytes, recall,
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
      Measure allocator/RSS and persisted/index bytes at multiple scales, fit a
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
