# GOAP World State — canonical current state

> **Compacted 2026-08-08 (ADR-0097).** This file holds **current truth only**:
> flags an agent needs to plan the next action. Historical per-wave/PR
> narrative (2026-02 → 2026-08) is archived verbatim at
> `plans/.archive/2026-08-08-historical/GOAP_STATE_2026_08_08.md` and
> `plans/.archive/2026-07-20-historical/`; full history is also in git.
>
> Hygiene rules:
> - `action_last_completed` appears **exactly once** (last key in this file;
>   YAML last-key-wins makes earlier duplicates silently dead).
> - Dated snapshots (metrics, PR logs, wave reports) belong in the archive,
>   not here. Record only the *current* value with a short dated comment.
> - When a flag flips, update it in place; do not append a new block.

world_state:
  # ── Core project ──────────────────────────────────────────────
  project_initialized: true
  dependencies_added: true
  core_modules_created: true
  tests_passing: true
  benchmarks_exist: true
  wasm_compiles: true
  binary_built: true
  documentation_complete: true
  validated: false               # Wave 32/33 remainder: ownership + evidence tiers still queued
  ci_all_checks_passed: true     # 2026-08-08: main HEAD e5affb9 green (release + ci)
  loc_gate_verified: true        # all first-party src/ and crates/ files ≤ 500 LOC

  # ── Canonical metrics (update in place with date comment) ────
  product_version: "0.3.8"       # crates.io 0.3.6/0.3.7/0.3.8 all published
  main_head: "46c2182"           # 2026-08-12
  tests_count: 1131              # 2026-08-11: literal #[test]/#[tokio::test] across src/crates/tests (incl. new persistence-disabled gates)
  skills_count: 32               # find .agents/skills -name SKILL.md | wc -l
  coverage_ratio_current: 93     # test:source ratio (target ≥90%)
  adr_registry_count: 94         # 2026-08-12: check-adr-parity.sh ok (registry=94, disk=93, 0003 N/A)
  adr_disk_count: 93
  integration_test_files: 70     # tests/*.rs (2026-08-11: +persistence_disabled.rs)

  # ── Plans pointers ────────────────────────────────────────────
  plans_active_index: "plans/README.md"
  plans_recommendations_canonical: "plans/RECOMMENDATIONS_2026_07_20.md"
  plans_archive_roots:
    - "plans/.archive/2026-07-20-historical"
    - "plans/.archive/2026-08-08-historical"
  active_plan_set_compact: true
  plan_archive_manifest_valid: true

  # ── Active wave ───────────────────────────────────────────────
  active_wave: 33
  wave_32_status: in_progress    # P0/P1 landed; ownership + evidence remain (see queued actions)
  wave_32_roadmap: "plans/GOAP_AUDIT_2026_07_14.md"
  wave_33_status: in_progress    # docs truth + missing behavior + evidence; mostly landed
  queued_actions_count: 4        # 2026-08-12: 4 active after deduplicate_persistence_owner_bodies rename

  # ── Open work (flags currently false — the real backlog) ──────
  no_missing_implementations: true            # 2026-08-12: no TODO in src/ crates/
  workspace_implementation_owners_unique: true    # 2026-08-12: csm-cli/csm-wasm dead dupes removed (#626/#627); root owns bodies, crates own binaries/facades
  no_default_features_is_lean: true               # 2026-08-08: no libsql/rayon in no-default tree (ADR-0094)
  msrv_workspace_aligned: true                    # 2026-08-08: all manifests use workspace rust-version 1.88
  persistence_disabled_false_success_removed: true  # 2026-08-11: replace_persistence_disabled_noops (ADR-0094); CLI DB config rejected w/o feature
  wasm_ci_release_artifact_identical: false
  performance_claims_have_current_artifacts: false # queued: scale benches + memory model
  critical_skill_evals_passing: false             # behavioral evals deferred
  fuzz_short_runs_on_pr: true                     # fuzz.yml fuzz-short job: 30s runs of changed targets on PRs
  fuzz_scheduled_full_runs: true                  # fuzz.yml fuzz-full job: weekly cron (Sun 03:00 UTC), 5min/target
  duckdb_companion_published: false               # csm-duckdb not on crates.io
  benchmarks_prove_performance: false             # evidence tiers pending (ADR-0095)
  deferred_namespace_isolation: false             # ADR-0026 multi-tenancy (trigger: user demand)
  deferred_phase2_optimizations: false            # ADR-0024 (trigger: >200k concepts + latency issues)

  # ── Landed invariants worth checking before changes ───────────
  retrieval_implementation_owner_unique: true     # 2026-07-23: csm-retrieval owns contracts
  ann_snapshot_revision_validated: true           # ADR-0093: IndexSnapshotEnvelope + ns revision
  ann_config_is_fallible: true                    # validate_index_backend; ADR-0093
  persistence_failure_leaves_memory_unchanged: true # durable commit before memory mutate
  mcp_full_width_vector_wire_contract: true       # base64 1280-byte HVec + high-bit tests
  public_f32_apis_validate_input: true            # 2026-08-07: PR #607 prune/neighbors validation
  workspace_ci_matrix_complete: true              # csm-chaos + benchmark tests in CI
  cargo_deny_required_in_ci: true
  fuzz_build_required_in_ci: true
  skill_validation_fail_closed: true              # wired into validate.sh + CI + pre-commit
  mutation_ci_enforced: true
  mutation_threshold: 85
  actions_pinned_to_sha: true
  harness_msrv_current: "1.88"
  dependabot_alerts_open: 5       # 2026-07-11 snapshot; all blocked upstream, documented in deny.toml

  # ── GOAP bookkeeping ──────────────────────────────────────────
  goap_reconciliations_complete:
    - adr_0084_2026_05_20
    - adr_0085_2026_06_06
    - adr_0089_2026_06_16
    - adr_0092_2026_07_11
    - adr_0097_2026_08_08
  goap_state_duplicate_key_fixed: true  # benchmark_workspace_tests_run_in_ci dup removed 2026-08-08

  # Must remain the LAST key and appear exactly once (see header).
  action_last_completed: reconcile_pr_wave_2026_08_12
