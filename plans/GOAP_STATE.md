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
  validated: false               # 2026-09-30: wave-32/33 exits re-verified vs GOAP_AUDIT_2026_07_14.md; residuals queued — TTL shutdown, absence invalidation, failure-path/query-count tests, gate+catalog consolidation, scheduled/release evidence tiers
  ci_all_checks_passed: true     # 2026-10-03: main push run 37121025587 (63f19bb, #815) green; predecessor run 37118324889 (f999998, #813) green incl. the Build CLI matrix
  loc_gate_verified: true        # all first-party src/ and crates/ files ≤ 500 LOC

  # ── Canonical metrics (update in place with date comment) ────
  product_version: "0.3.8"       # crates.io 0.3.6/0.3.7/0.3.8 all published
  main_head: "63f19bb"           # 2026-10-03: #815 close-out of wire_graceful_shutdown_into_servers (latest main at record time)
  tests_count: 1037              # 2026-09-30: unique compiled behavior (scripts/coverage-report.sh inventory)
  skills_count: 33               # 2026-09-07: +pr-roast-triage (find .agents/skills -name SKILL.md | wc -l)
  coverage_lines_percent: 74     # 2026-09-18: cargo +nightly llvm-cov --workspace --lib --tests --branch
  coverage_branches_percent: 64  # same run; unit-only targets measure 68/53, hence --tests matters
  adr_registry_count: 95         # 2026-10-01: check-adr-parity.sh ok (registry=95, disk=94, 0003 N/A)
  adr_disk_count: 94
  integration_test_files: 72     # tests/*.rs (2026-09-30 recount)

  # ── Plans pointers ────────────────────────────────────────────
  plans_active_index: "plans/README.md"
  plans_recommendations_canonical: "plans/RECOMMENDATIONS_2026_07_20.md"
  plans_archive_roots:
    - "plans/.archive/2026-07-20-historical"
    - "plans/.archive/2026-08-08-historical"
  active_plan_set_compact: true
  plan_archive_manifest_valid: true  # 2026-09-30: ARCHIVE_MANIFEST.md + README redirects exist; no validator reads it and 55 top-level archived ADRs are unlisted (queued)

  # ── Active wave ───────────────────────────────────────────────
  active_wave: 33
  wave_32_status: in_progress    # 2026-09-30: exits re-verified — ownership/features/scale-evidence/metres landed; residuals queued (TTL shutdown, absence invalidation, failure-path + query-count tests, gate + catalog work, evidence tiers)
  wave_32_roadmap: "plans/GOAP_AUDIT_2026_07_14.md"
  wave_33_status: in_progress    # docs truth + missing behavior + evidence; mostly landed
  queued_actions_count: 8        # 2026-10-03: collapse_duplicate_concept_builder completed (9 → 8). Counted with `grep -c '^  - name:' plans/ACTIONS.md`, not asserted — the 2026-10-03 round found a bookkeeping PR whose number was right only by coincidence of two other PRs.

  # ── Open work (flags currently false — the real backlog) ──────
  no_missing_implementations: true            # 2026-08-12: no TODO in src/ crates/
  workspace_implementation_owners_unique: true    # 2026-08-12: csm-cli/csm-wasm dead dupes removed (#626/#627); root owns bodies, crates own binaries/facades
  no_default_features_is_lean: true               # 2026-08-08: no libsql/rayon in no-default tree (ADR-0094)
  msrv_workspace_aligned: true                    # 2026-08-08: all manifests use workspace rust-version 1.88
  persistence_disabled_false_success_removed: true  # 2026-08-11: replace_persistence_disabled_noops (ADR-0094); CLI DB config rejected w/o feature
  wasm_ci_release_artifact_identical: true  # 2026-09-20: CI builds + smoke-tests the release/web package via scripts/build-wasm.sh (same command release.yml publishes); size gate measures that artifact (656 657 B, sha256-pinned)
  performance_claims_have_current_artifacts: true  # 2026-09-17: plans/evidence/scale_2026_09_17 (three manifests, #733/#734)
  critical_skill_evals_passing: false             # behavioral evals deferred
  fuzz_short_runs_on_pr: true                     # fuzz.yml fuzz-short job: 30s runs of changed targets on PRs
  fuzz_scheduled_full_runs: true                  # fuzz-full weekly cron (Sun 03:00 UTC); nightly toolchain + nix shell dropped (PR #690); first scheduled green 2026-09-13 (run 34746887403) after red ×6 (08-02..09-06)
  duckdb_companion_published: true                # 2026-09-27: csm-duckdb 0.3.8 first-published (owner d-o-hub); release.yml lockstep integration queued
  bhvec_xor_simd_measured_slower: true # 2026-10-02: #808 closed as no-impact — main scalar 89.5 ns vs SIMD dispatch 113.3 ns (i5-8350U, AVX2 present, rustc 1.88.0); the dispatch adds memset + memcpy + an out-of-line target_feature call
  benchmarks_prove_performance: true              # 2026-09-21: named runner (plans/REFERENCE_RUNNER.md), canonical criterion baseline (plans/evidence/bench/canonical.json, 88 benches + CI), release-scale evidence (plans/evidence/scale_release_2026_09_21: ANN to 200k, memory/storage to 500k), npm package evidence (plans/evidence/wasm_2026_09_21)
  perf_pr_evidence_gate_enforced: true           # 2026-09-23: ci.yml commitlint job runs scripts/check-perf-pr-evidence.py for `perf(...)` PR titles; PR template carries `## Performance Evidence`
  deferred_namespace_isolation: false             # ADR-0026 multi-tenancy (trigger: user demand)
  deferred_phase2_optimizations: false            # ADR-0024 (trigger: >200k concepts + latency issues)

  # ── Landed invariants worth checking before changes ───────────
  retrieval_implementation_owner_unique: true     # 2026-07-23: csm-retrieval owns contracts
  ann_scale_evidence_current: true                # 2026-09-17: evidence_ann.json (exact/hnsw/lsh/bucket at 1k/10k/50k)
  persistence_contention_evidence_current: true   # 2026-09-17: evidence_persistence.json (error rate 0.905 -> 0.000)
  measured_memory_model_exists: true              # 2026-09-17: 4691 B RSS + 2850 B storage per concept, held-out 0.29 %
  canonical_test_owners_unique: true              # 2026-09-18: facade test copies of owner tests removed; audit in plans/TEST_SURFACE_AUDIT_2026_09_18.md
  coverage_methodology_behavior_based: true       # 2026-09-18: unique compiled behavior + llvm-cov line/branch, not test LOC
  bucketed_recall_scales_with_corpus: true        # 2026-09-21: adaptive multi-probe + budget guard; probe declines to the exact scan instead of truncating
  ten_million_memory_claim_evaluated: true        # 2026-09-17: evaluated NOT supported (43.7 GB RSS / 26.5 GB storage)
  ann_snapshot_revision_validated: true           # ADR-0093: IndexSnapshotEnvelope + ns revision
  ann_config_is_fallible: true                    # validate_index_backend; ADR-0093
  persistence_failure_leaves_memory_unchanged: true # 2026-09-30: durable-first order + reload reconcile (framework_persistence.rs:244-296); failure path untested (queued)
  persistence_implementation_owner_unique: true  # 2026-09-15: root facade re-exports csm-persistence (phases 1-3); no second body
  mcp_full_width_vector_wire_contract: true       # base64 1280-byte HVec + high-bit tests
  no_state_lock_across_io_await: true             # 2026-09-30: durable I/O before singularity locks (framework_persistence.rs:96,177-215), verified by inspection
  benchmark_metrics_mathematically_correct: true  # 2026-09-30: multi-label recall_at_k, log2 NDCG, abstention gold from should_abstain + hand-calculated tests (benchmarks/src/scorer.rs)
  public_f32_apis_validate_input: true            # 2026-08-07: PR #607 prune/neighbors validation
  workspace_ci_matrix_complete: true              # csm-chaos + benchmark tests in CI; crate list hand-maintained (ci.yml:202-241, machine-derived matrix queued)
  cargo_deny_required_in_ci: true
  fuzz_build_required_in_ci: true
  release_wait_event_driven: true                 # 2026-09-30: release.yml triggers on CI completion (workflow_run, head_sha-pinned); no polling ceiling. Proof: runs 36746741481 + 36746882951 green, publishes correctly skipped
  crates_publish_precheck_ownership_aware: true   # 2026-09-30: owners API (free or d-o-hub passes; other owners fail; unknown HTTP fails closed) replaces the published-version comparison; verified live incl. negative control
  csm_duckdb_in_release_publish_order: true       # 2026-09-30: dedicated step after the root publish waits for the root version in the index; companion loop exposes failures instead of swallowing them
  ttl_cleanup_has_bounded_shutdown: true          # 2026-10-01: ADR-0099 — shared CleanupTask, cooperative watch cancel (no abort), bounded await in shutdown(); loop also stops on last-handle drop. 20 TTL + 2 unit tests; mutation-test green. 2026-10-03 (#813, f999998): `shutdown()` now actually runs on the `csm watch` SIGINT path and after the `mcp::serve` transport ends; that call-site wiring rests on review + a manual SIGINT smoke — the `run_watch -> Ok(())` entry-point mutant is excluded in `scripts/mutation_test.sh`, and neither path has cleanup enabled yet (see `enable_ttl_cleanup_in_long_running_commands`)
  absence_records_invalidated_on_insert: true     # 2026-10-01: absence rows carry namespace + revision, so any durable mutation (revision bump, ADR-0093) makes them stale; successful probes delete the record. Migration v12; regression + 100% mutation score
  ttl_clamped_on_the_live_builder: true              # 2026-10-02 (#806, 8f9dc3d): the generic builder production actually binds (singularity_types.rs) now clamps to MAX_TTL_SECONDS_LIMIT; superseded by concept_builder_owner_unique below (one clamp site now)
  concept_builder_owner_unique: true                 # 2026-10-03: the `singularity_types::ConceptBuilder` copy is deleted, `csm_memory::singularity::ConceptBuilder` is a `pub use` of the `concept_builder` owner, so one struct and one `MAX_TTL_SECONDS_LIMIT` clamp serve every path. Guarded by two `tests/arch_fitness.rs` checks (single struct definition; five public spellings unify on the owner type). Not breaking — no `Clone` call sites existed; `with_metadata` now takes `impl Serialize` with ADR-0012 error capture everywhere.
  skill_validation_fail_closed: true              # wired into validate.sh + CI + pre-commit
  llms_dependency_versions_current: true          # 2026-09-27: llms.txt/llms-full.txt regenerated (otel 0.32, rmcp 3.4); scripts/check-llms-sync.sh drift gate runs in validate.sh (CI lint job)
  retrieval_string_clones_removed: true           # 2026-09-27: #781 borrowed expansion ids/labels + positions scoring; 2209 -> 151 allocs per query (top_k=10)
  retrieval_cache_invalidated_on_mutation: true   # 2026-09-29: insert/upsert and association mutations invalidate cached results; regression-covered
  capacity_upsert_preserves_existing: true        # 2026-09-29: existing IDs are detected before max_concepts eviction; regression-covered
  mutation_ci_enforced: true
  mutation_threshold: 85
  actions_pinned_to_sha: true
  harness_msrv_current: "1.88"
  adr_0095_accepted: true        # 2026-09-17: plans/adr/0095-evidence-driven-quality-gates.md is Accepted (2026-07-16); precondition of deduplicate_test_and_source_surfaces
  dependabot_alerts_open: 3       # 2026-09-17: otel GHSA cleared (#729); left: lru (quinn-proto/tantivy pin ^0.12) + libsql-sqlite3-parser ×2 (no fix, denied in deny.toml)

  # ── GOAP bookkeeping ──────────────────────────────────────────
  goap_reconciliations_complete:
    - adr_0084_2026_05_20
    - adr_0085_2026_06_06
    - adr_0089_2026_06_16
    - adr_0092_2026_07_11
    - adr_0097_2026_08_08
  goap_state_duplicate_key_fixed: true  # benchmark_workspace_tests_run_in_ci dup removed 2026-08-08

  # Must remain the LAST key and appear exactly once (see header).
  action_last_completed: collapse_duplicate_concept_builder
