world_state:
  project_initialized: true
  dependencies_added: true
  core_modules_created: true
  tests_passing: true
  benchmarks_exist: true
  wasm_compiles: true
  binary_built: true
  sample_app_created: true
  documentation_complete: true
  validated: true
  dependency_hygiene_complete: true
  adr_registry_current: true
  result_contract_clarified: true
  architecture_docs_two_tier: true
  architecture_docs_canonical_source: "context.yaml"
   action_last_completed: inline_tests_93_coverage_2026_04_30
  orchestrator_last_run: goap_orchestrator_analysis_2026_04_30
  orchestrator_last_run_at_utc: 2026-04-30T18:30:00Z
  goap_orchestrator_analysis_complete: true         # 2026-04-30: All actionable items verified complete
  skills_all_defined: true                          # 2026-04-30: 29 skills with SKILL.md verified
  no_missing_implementations: true                  # 2026-04-30: No TODO/FIXME/unimplemented! in source
  book_chapters_complete: true                      # 2026-04-30: 15 book chapters verified (semantic-bridge, inertial-reservoir, ttl exist)
  changelog_links_complete: true                    # 2026-04-30: All version links verified
  hybrid_retrieval_example_compiles: true           # 2026-04-30: cargo check --example hybrid_retrieval OK
  loc_gate_verified: true                           # 2026-04-30: All files ≤500 LOC (max: singularity.rs, wasm_ext.rs at 500)
  misleading_docs_found: true                       # 2026-04-30: README.md WASM issue outdated (lines 500-503)
  # PR Status
  pr_138_merged: true                           # 2026-04-30: Coverage improvements + WASM fix (571 tests, 70%)
  pr_138_tests_added: 45                        # encoder(7), persistence_ops(3), framework_ttl(9), wasm_ext(17), skill-memory(17)
  pr_138_wasm_export_import_fix: true           # BinaryExportPayload for bincode serialization
  pr_130_codex_p1_fixed: true                   # 2026-04-29: All 4 P1 issues addressed before merge
  pr_130_sha_validation_opt_in: true            # pre-commit.sh: CSM_VALIDATE_GITHUB_ACTIONS_SHAS=true
  pr_130_ci_sha_binding: true                   # self-fix-loop.sh: captures pushed SHA, filters by headSha
  pr_130_subpath_regex: true                    # validate-github-actions-shas.sh: supports owner/repo/path@ref
  pr_130_yaml_extension: true                   # validate-github-actions-shas.sh: finds .yaml files
  pr_129_merged: true                           # 2026-04-29T07:52:50Z squash-merged → 787098a
  perf_singularity_integer_scoring_landed: true # Hamming int ranking + fused (idx,score) + Rayon with_min_len(512)
  # Real-usage verification (2026-04-29) — see plans/VERIFICATION_2026_04_29.md
  verification_2026_04_29_completed: true
  verification_examples_passing: 7              # 7/7 examples end-to-end OK
  verification_criterion_benches_passing: 3     # bm25_benchmark, persistence_benchmark, benchmark
  verification_workspace_recall_at_1: 0.75
  verification_workspace_mrr: 0.75
  verification_bm25_search_1000_us: 64.4        # was 3030 µs pre PR #129 (47× faster)
  verification_bridge_retrieval_pipeline_1k_ms: 1.92
  tests_count: 598                              # 2026-04-30: +27 inline tests (reservoir:7, singularity:5, retrieval:6, ops:3, wasm:4)
  skills_count: 29                              # 2026-04-30: All 29 skills have SKILL.md verified
  coverage_ratio_current: 93                   # Test:Source ratio (target: 90% - ACHIEVED)
  coverage_inline_tests_added: true             # encoder.rs (7), persistence_ops.rs (3), framework_ttl.rs (9), wasm_ext.rs (17)
  coverage_core_inline_tests_added: true       # 2026-04-30: reservoir.rs (7), singularity.rs (5), singularity_retrieval.rs (6), framework_ops.rs (3), wasm.rs (4)
  coverage_skill_memory_integration_added: true # 17 tests for CLI patterns (tests/skill_memory_integration.rs)
  coverage_wasm_export_import_fix: true         # BinaryExportPayload regression tests (export_payload_tests.rs)

  # ═══════════════════════════════════════════════════════
  # Production Readiness (Real Usage Test 2026-04-30)
  # ═══════════════════════════════════════════════════════
  production_rust_library_ready: true           # Tested from crates.io v0.3.5
  production_cli_ready: true                    # All CLI commands pass
  production_wasm_export_import_bug: false      # FIXED in PR #138
  wasm_export_uses_binary_payload: true        # Now uses BinaryExportPayload for bincode
  wasm_export_fix_verified: true                # Regression tests pass
  production_score: 8.5                         # Up from 7.5 - WASM fixed, coverage 70%
  # Remaining observations addressed (2026-04-29)
  latency_reporting_uses_us_for_sub_ms: true    # p50_latency_us field added to benchmarks
  hybrid_retrieval_example_exists: true         # examples/hybrid_retrieval.rs created
  ci_all_checks_passed: true                  # 2026-04-28: Release workflow fixed + v0.3.5 published
  ci_release_workflow_blocked: false           # 2026-04-28: Fixed - checkout + retry logic
  ci_release_workflow_fix_applied: true        # 2026-04-28: actions: read + null guards
  ci_pages_workflow_fix_applied: true          # 2026-04-28: heredoc terminator de-indented
  ci_pre_release_gate_failing: true            # version triad + security audit + planning
  changelog_v033_yanked_recorded: true         # 2026-04-28: backfilled CHANGELOG
  changelog_unreleased_section: true           # 2026-04-28: added Unreleased
  goap_actions_md_synced: true                 # 2026-04-28: 3 queued -> complete

  # GitHub Actions Security Audit (2026-04-21)
  codeql_missing_permissions_fixed: true
  codeql_alerts_resolved: 4  # actions/missing-workflow-permissions in ci.yml
  dependabot_alerts_open: 6  # rustls-webpki(3), rand(2), libsql-sqlite3-parser(1)
  dependabot_blocked_upstream: true  # No stable rustls-webpki >=0.103.10 exists

  # ═══════════════════════════════════════════════════════
  # Research-driven enhancements (2026-04-20)
  # Source: plans/RESEARCH_2026_PAPERS.md
  # ═══════════════════════════════════════════════════════

  # HIGH-1: InertialESN — Second-order reservoir dynamics
  # Paper: Zhao et al., "Inertial ESN", Neurocomputing Apr 2026
  # State update: state[t] = (1-α)·state[t-1] + α·tanh(W·state+W_in·input) + β·(state[t-1]-state[t-2])
  # Also: deterministic mixing operator replaces random sparse matrix
  inertial_reservoir_adr_written: true
  inertial_reservoir_implemented: true
  inertial_reservoir_tested: true  # tests/reservoir_inertial_tests.rs implemented 2026-04-21
  inertial_reservoir_benchmarked: true  # benches/benchmark.rs: 4 bench groups (step_beta0/beta015, sequence_10_beta0/beta015)

  # HIGH-2: Selectivity-aware filtered retrieval
  # Paper: Amanbayev et al., "Filtered ANN Search", arXiv:2602.11443 Feb 2026
  # Current find_similar_filtered() already pre-filters (line 122 singularity_ext.rs)
  # Enhancement: selectivity-adaptive strategy (bucket vs graph vs scan) based on
  # filter selectivity ratio = filtered_count / total_count
  selectivity_aware_retrieval_adr_written: true
  selectivity_aware_retrieval_implemented: true
  selectivity_aware_retrieval_tested: true  # tests/retrieval_selectivity_tests.rs implemented 2026-04-21

  # ═══════════════════════════════════════════════════════
  # Wave 20 Queue Validation (2026-04-29)
  # All 16 queued items reviewed; 14 already complete, 2 blocked/deferred
  # ═══════════════════════════════════════════════════════
  wave_20_queue_validation_completed: true
  wave_20_queue_results:
    # Dependency upgrades (all already current)
    IQ_01_libsql_upgrade: complete        # Already at 0.9.30 with Builder API
    IQ_02_rand_upgrade: complete          # Already at 0.10.1
    IQ_03_getrandom_wasm_flag: complete   # Already at 0.4.2 + wasm_js feature
    IQ_04_bincode_to_postcard: deferred   # No user demand; security limits already present
    # Skill-memory hardening (all implemented)
    IQ_05_retry_logic: complete           # _cli_with_retry in skill-memory.sh
    IQ_06_health_checks: complete         # skill_memory_check function
    IQ_07_metrics_collection: complete    # _metrics_record in skill-memory-metrics.sh
    IQ_08_log_rotation: complete          # _rotate_logs in skill-memory.sh
    IQ_09_encryption: complete            # _encrypt_data/_decrypt_data in skill-memory-advanced.sh
    IQ_10_rate_limiting: complete         # CSM_RATE_LIMIT_* config
    # npm publishing (blocked)
    IQ_11_npm_oidc: blocked               # Requires external account configuration
    IQ_12_npm_publish_automate: blocked   # Depends on IQ-11
    # Error handling (already complete)
    IQ_13_error_source_attributes: complete  # #[source] on Database/Reservoir variants
    IQ_14_error_context_hints: deferred      # No user demand; marked deferred
    IQ_15_security_tests: deferred           # Property tests exist; no dedicated security suite
    IQ_16_avx2_neon_simd: complete           # AVX2 + NEON paths verified in hyperdim_simd.rs

  # ═══════════════════════════════════════════════════════
  # Coverage Analysis (2026-04-30)
  # 571 tests, 8501 LOC tests, 12140 LOC source (ratio: 0.70:1 inline tests separate)
  # All modules have dedicated test coverage
  # ═══════════════════════════════════════════════════════
  coverage_analysis_completed: true
  coverage_metrics:
    total_tests: 571
    test_files: 46
    source_files: 55
    test_loc: 8501
    source_loc: 12140
    inline_test_loc: 2333            # Tests embedded in src modules
    test_to_source_ratio: 0.70       # Excluding inline tests from source LOC
    inline_test_modules: 31          # Up from 27 (encoder, persistence_ops, framework_ttl, wasm_ext, export_payload_tests)
    dead_code_markers: 4             # BinaryConcept serialization fields
    todo_markers: 0
  coverage_gaps_identified: 5        # See plans/GOAP_COVERAGE_IMPROVEMENT.md
  coverage_plan_created: true        # plans/GOAP_COVERAGE_IMPROVEMENT.md
  coverage_gaps_resolved: 5          # 2026-04-30: All 5 gaps addressed with inline tests
  coverage_pr_138_merged: true       # 2026-04-30: PR #138 merged with +45 tests, coverage 66→70%

  # ═══════════════════════════════════════════════════════
  # Clippy Best Practices (2026-04-29)
  # Passing with -D warnings, 6 justified #[allow(...)]
  # ═══════════════════════════════════════════════════════
  clippy_analysis_completed: true
  clippy_status: passing
  clippy_per_item_allows: 6       # dead_code (4), too_many_arguments (1), type_complexity (1), needless_range_loop (1)
  clippy_config_file_exists: true     # .clippy.toml created 2026-04-30
  cargo_lints_section_exists: true    # [lints.rust] + [lints.clippy] added 2026-04-30
  clippy_best_practices_plan_created: true  # plans/GOAP_CLIPPY_BEST_PRACTICES.md
  clippy_inline_tests_added: true     # 2026-04-30: hyperdim_simd (6), reservoir_sparse (10), persistence_wasm (4), framework_events (5), singularity_cache (9)

  # Benchmark Optimization Analysis (2026-04-09) - COMPLETE
  benchmark_optimization_analysis_active: false
  benchmark_optimization_analysis_completed: true
  benchmark_optimization_implementation_status:
    phase_1_correctness: completed
    phase_2_metrics: completed
    phase_3_scorer: completed
    phase_4_parallel_ingest: not_needed  # Benchmark data shows avg 2.5ms/session < 5ms threshold
    phase_5_generator: completed
    phase_6_measurement: completed
  benchmark_optimization_phase4_evaluation:
    decision: "Not needed - sequential ingest is adequate"
    benchmark_results:
      sessions_10:
        ingest_ms: 38
        avg_ms_per_session: 3.8
      sessions_100:
        ingest_ms: 261
        avg_ms_per_session: 2.6
      sessions_500:
        ingest_ms: 1258
        avg_ms_per_session: 2.5
    threshold: "5ms avg per session"
    conclusion: "Sequential ingest performs well under threshold at all tested scales"
  benchmark_optimization_files_changed:
    - benchmarks/src/cli.rs
    - benchmarks/src/generator.rs
    - benchmarks/src/main.rs
    - benchmarks/src/metrics.rs
    - benchmarks/src/report.rs
    - benchmarks/src/runner.rs
    - benchmarks/src/scorer.rs
    - benchmarks/src/types.rs
  benchmark_optimization_loc_added: 227
  benchmark_optimization_tests_added: 5
  benchmark_optimization_tests_passing: 19

  # Benchmark Verification (2026-04-09) - COMPLETE
  benchmark_verification_completed: true
  benchmark_verification_date: "2026-04-09T15:50:00Z"
  benchmark_verification_ci_status:
    ci_workflow: success
    release_workflow: success
    benchmark_ci_workflow: success
  benchmark_verification_results:
    bm25_search_1000_ms: 3.03
    bm25_replace_doc_ns: 463
    core_tests_passed: 4
    benchmark_tests_passed: 19
  benchmark_verification_conclusion: "All optimizations validated - CI passing, tests passing, benchmarks within targets"

  # Repository analysis snapshot (2026-04-12)
  repo_analysis_2026_04_12_completed: true
  repo_analysis_2026_04_12_scope:
    - missing_implementations
    - tests
    - evals
    - benchmarks
    - github_actions
  repo_analysis_2026_04_12_findings:
    ci_benchmark_executes_criterion_targets: true
    benchmark_workspace_tests_in_ci: true
    wasm_js_smoke_test_in_ci: true
    wasm_docs_match_generated_api: true
    benchmark_ci_quality_thresholds_enforced: true
    benchmark_storage_metric_truthful: true
    benchmark_report_contract_complete: true
    pages_fallback_emits_html: true
  repo_analysis_2026_04_12_validated_commands:
    - "cargo test --all-features --quiet"
    - "cargo check --target wasm32-unknown-unknown --features wasm"
    - "cargo run --manifest-path benchmarks/Cargo.toml -- --dataset-dir benchmarks/datasets/v1/small --mode retrieval-only --out-dir benchmarks/results/ci"
    - "cargo test --manifest-path benchmarks/Cargo.toml --quiet"
    - "cargo bench --lib --no-fail-fast"
  repo_analysis_2026_04_12_notes:
    - "Main CI benchmark command only ran bench-profile unit tests; Criterion bench targets in benches/*.rs were not executed."
    - "Benchmark CI passed despite poor retrieval metrics because it validates artifact presence instead of metric thresholds."
    - "WASM Rust target compiles, but the documented JS API and wasm/test.js drift from the generated pkg API and are not exercised in CI."
    - "Benchmark runner reports dataset file size as storage_bytes and always uses the in-memory adapter."
  ci_executes_real_criterion_benches: true
  benchmark_workspace_tests_run_in_ci: true
  wasm_js_smoke_test_enforced: true
  benchmark_ci_enforces_quality_thresholds: true
  benchmark_storage_metric_truthful: true
  benchmark_report_contract_complete: true
  pages_fallback_renders_html: true
  wasm_docs_match_generated_api: true

  # Recent changes (consolidated)
   recent_changes:
     - "Fixed gen-llms-txt.sh to only generate files without auto-adding to git (prevents release workflow state issues)"
     - "Added llms.txt generation step to release.yml validate job for AI tool integration"
     - "Documentation audit: fixed 21 discrepancies across 9 .md files against codebase"
    - "Fixed: ConceptBuilder::build() returns Result (README Quick Start)"
    - "Fixed: concept_cache_size default is 128 not 1000 (README, book, pkg)"
    - "Fixed: HVec10240 API signatures - random() takes no params, bind/cosine_similarity are instance methods"
    - "Fixed: ConceptBuilder::with_metadata takes (key, value) not single JSON"
    - "Fixed: MemoryError::NotFound variant (not ConceptNotFound)"
    - "Fixed: with_turso method (not with_remote_db)"
    - "Fixed: Removed non-existent CSM_* environment variables"
    - "Fixed: Singularity not in prelude (graph.md)"
    - "Fixed: CLI exit codes match actual ExitCode enum; --vector flag replaced with --from-file"
    - "Fixed: WASM class is WasmFramework (not ChaoticSemanticFramework); score not similarity"
    - "Fixed: WASM build requires --no-default-features"
    - "Fixed: CHANGELOG.md missing version links for 0.2.3, 0.2.4, 0.2.5"
    - "Added Documentation Accuracy Checklist to LEARNINGS.md"
    - "PR #34: Extracted retrieval optimization code to singularity_retrieval.rs to satisfy LOC gate"
    - "singularity.rs now 484 LOC (was 758), singularity_retrieval.rs added at 292 LOC"
    - "All tests passing, CI LOC gate satisfied"
    - "Closed issues #19-#28 after verifying framework API, WASM bindings, and error-chain support on main"
    - "PR #31 merged; release workflow validated changelog/doc sync on main"
    - "Release workflow skipped publish because v0.2.2 tag already exists"
    - "Release workflow now runs on main merge, creates tag + GitHub Release in one flow"
    - "Release summary step fixed (closing fi, skip summary when tag exists)"
    - "npm publish guard now checks registry version list before attempting publish"
    - "Release CI fix: tag builds no longer attempt git push from detached HEAD"
    - "npm publish guard: skip already-published versions; workflow_dispatch rerun succeeded"
    - "Release v0.2.2 complete: crates.io + GitHub Release ok; npm publish idempotent"
    - "Release prep: synced version to v0.2.2 and refreshed Cargo.lock dependencies"
    - "CI monitoring: LOC gate failure traced to persistence.rs in older runs; validation now passes"
    - "CodeQL completed on main/PR; Node.js 20 deprecation warning recorded for workflow follow-up"
    - "Phase 41 follow-up: split persistence_versions to satisfy LOC gate"
    - "Phase 41 follow-up: WASM traversal config now uses default max_results"
    - "Phase 41 follow-up: WASM metadata updates emit MemoryEvent"
    - "Phase 48 complete: probe cache-miss scan no longer materializes cloned concept vectors"
    - "Local SQLite path now enforces PRAGMA journal_mode=WAL while preserving per-connection FK ON"
    - "Added Phase 48 coverage: WAL mode tests + 10k/100k/200k exact probe benchmark group"
    - "Fixed WASM metadata JSON fidelity via js_sys::JSON::parse in concept conversion"
    - "Added book chapters: encoder.md and graph.md, linked from mdBook SUMMARY"
    - "v0.2.0 released: crates.io ✅, npm ✅, GitHub Release ✅"
    - "Wave 16 complete: Production Polish & Correctness (ADR-0055)"
    - "FNV-1a hash stability in TextEncoder (breaking: re-encode persisted vectors)"
    - "Weighted Dijkstra shortest_path + shortest_path_hops BFS compat"
    - "BundleAccumulator::try_remove (no panic), remove saturates at zero"
    - "Reservoir::to_hypervector panic replaced with MemoryError::Reservoir"
    - "WASM parity: update_concept_metadata, clear_associations, neighbors, bfs, shortest_path"
    - "21 new Wave 16 tests, 4 new benchmark groups"
    - "Merged PR #15: Wave 15 complete"
    - "Completed Wave 15: API Hardening & New Features"
    - "Created ADR-0053: API Hardening & Missing Features (Implemented)"
    - "Created ADR-0054: High-Impact New Features (Implemented)"
    - "Added metadata_filter.rs: predicate-based filtering for similarity search"
    - "Added graph_traversal.rs: BFS, shortest_path, neighbors, incoming_associations"
    - "Added BundleAccumulator to hyperdim.rs for streaming/sliding-window memory"
    - "Added find_similar_filtered to singularity for RAG patterns"
    - "Added TextEncoder (encoder.rs): deterministic text-to-hypervector encoding"
    - "Added inject_text/probe_text convenience methods to framework"
    - "Added WASM bindings for inject_text, probe_text, encode_text"
    - "All tests passing, clippy clean, fmt clean"

  # PR Status
  pr_15_merged: true
  pr_15_merged_at: "2026-03-03T10:30:00Z"
  
  # CI Status
  ci_main_passing: true
  codeql_main_passing: true

  # Release Status (v0.2.0)
  release_v020_completed: true
  release_v020_completed_at: "2026-03-04T18:31:00Z"
  crates_io_v020_published: true
  npm_v020_published: true
  github_release_v020_created: true

  # Release Status (v0.3.2)
  release_v032_prepared: true
  release_v032_changes:
    - "Benchmark harness optimizations (v0.3.1, v0.3.2)"
    - "BM25 parallel scoring with Rayon"
    - "NDCG@k scoring, p99 percentile"
    - "Percentile indexing fix"
  crates_io_v032_ready: true
  wasm_size_gate_passed: true
  wasm_library_size_kb: 852
  all_tests_passing: true
  tests_count_v032_baseline: 333  # historical baseline; current count tracked in tests_count above

  # Swarm orchestration snapshot
  active_wave: 20
  wave_strategy: parallel_by_phase_with_handoffs
  wave_20_name: "Implementation Queue Rebuild"
  wave_20_started_at: "2026-04-12"
  wave_20_focus: "Queue reconstruction + A-F parallel assignment + handoff contracts"
  wave_20_queue_total: 16
  wave_20_queue_blocked: 2
  wave_20_queue_executable: 14
  wave_20_handoff_contracts_created: 5
  wave_20_status: queued
  # Completed waves 1-14, 17 — summary (detail in progress/WAVE_HISTORY.md)
  waves_1_through_9_complete: true   # Swarm phases: fuzz, SIMD, logging, migration, pooling, export, versioning, CLI
  waves_11_through_14_complete: true # Release engineering, security hardening, real-world readiness
  wave_17_complete: true             # Performance follow-up & GOAP consistency
  all_waves_finished: false
  swarm_status: active
  final_validation_passed: true

  # PR tracking
  pr_94_status: open
  pr_94_title: "perf(retrieval): optimize BM25 scoring loop"
  pr_94_ci_passed: true
  pr_94_source: jules_bot

  # Release orchestration (completed)
  release_orchestration_completed: true
  issue_50_status: closed
  crates_io_v028_published: true
  github_release_v028_created: true
  npm_v028_published: false
  npm_publish_blocked_reason: "OTP required - npm account requires one-time password authentication"
  npm_missing_versions: []  # 2026-04-28: v0.3.5 published to all registries
  npm_latest_published: "0.3.5"
  dependabot_alert_3: blocked_upstream
  dependabot_alert_3_package: rustls-webpki
  dependabot_alert_3_blocker: "No stable rustls-webpki >=0.103.10 exists"

  # Module status (LOC counts) - Updated 2026-04-21
  modules:
    lib.rs: 217
    error.rs: 32
    hyperdim.rs: 500
    reservoir.rs: 500
    reservoir_inertial.rs: 34
    singularity.rs: 447
    singularity_retrieval.rs: 405
    singularity_ext.rs: 234
    persistence.rs: 491
    persistence_ops.rs: 371
    persistence_wasm.rs: 122
    persistence_migrations.rs: 162
    framework.rs: 491
    framework_ops.rs: 476
    framework_builder.rs: 248
    framework_validation.rs: 184
    framework_bridge.rs: 176
    export_payload.rs: 174
    concept_builder.rs: 164
    encoder.rs: 329
    graph_traversal.rs: 460
    metadata_filter.rs: 200
    bundle.rs: 178
    wasm.rs: 469
    wasm_ext.rs: 265
    semantic_bridge.rs: 400
    bridge_retrieval.rs: 386
    bridge_persistence.rs: 293
    retrieval/bm25.rs: 471
    retrieval/hybrid.rs: 242
    cli/args.rs: 228
    cli/git_local.rs: 182
    cli/commands/mod.rs: 115
    cli/commands/query.rs: 282
    cli/commands/index_dir.rs: 304
    cli/commands/index_jsonl.rs: 153
    cli/commands/inject.rs: 177
    cli/commands/probe.rs: 154
    cli/commands/associate.rs: 163
    cli/commands/export.rs: 65
    cli/commands/import.rs: 128
    cli/commands/completions.rs: 45
    bin/csm.rs: 160

  # Test status - Updated 2026-04-21
  tests_passing: 333
  integration_tests_exist: true
  total_tests: 333

  # Correctness issues (must fix)
  permute_shift_zero_bug: false
  reservoir_to_hvec_div_zero: false
  reservoir_dense_matrix_infeasible: false
  associations_allow_duplicates: false
  load_silently_overwrites: false
  reservoir_not_reset_between_sequences: false
  prelude_module_missing: false

  # Performance gaps
  retrieval_latency_high: false
  singularity_search_sequential: false
  reservoir_step_per_alloc: false
  bundle_per_chunk_alloc: false
  persistence_no_batching: false
  persistence_connection_unsafe: false

  # Missing capabilities
  no_concept_deletion_in_framework: false
  no_memory_limits: false
  no_integration_tests: false
  wasm_rayon_not_gated: false
  sqlite_foreign_keys_not_enforced: false
  conceptbuilder_swallows_metadata_errors: false
  libsql_deprecated_apis_used: false
  debug_impls_missing: false  # Fixed in Wave 8

  # Validation outcomes
  wasm_target_installed: true
  reservoir_step_under_100us: true
  reservoir_step_50k_latest_us: 76.627

  # Batch similarity performance (ADR-0041) - Phase 2 Complete
  batch_similarity_under_500us: true
  batch_similarity_1000_latest_us: 470
  benchmarks_prove_performance: true
  turso_roundtrip_under_20ms: true
  10m_concepts_under_12mb: true
  wasm_binary_under_500kb: true
  no_hardcoded_runtime_settings: true
  no_magic_numbers_without_named_constants: true

  # Phase 5: Testing & Quality (cost: 8)
  property_based_tests_added: true
  fuzzing_targets_created: true
  edge_case_coverage_complete: true
  mutation_testing_enabled: true

  # Phase 6: Performance Enhancements (cost: 12)
  simd_hypervector_ops: true
  connection_pooling_turso: true
  framework_batch_operations: true
  concept_cache_implemented: true

  # Phase 7: Observability & DX (cost: 10)
  structured_logging_added: true
  metrics_collection_enabled: true
  derive_macros_created: false  # REMOVED: never used, eliminated to reduce maintenance
  error_context_improved: true

  # Phase 8: Advanced Features (cost: 15)
  export_import_functionality: true
  concept_versioning_enabled: true
  schema_migration_support: true
  backup_restore_operations: true
  concept_ttl: true                         # Baseline TTL APIs and expires_at persistence shipped

  # Phase 10: Hardening & Validation (cost: 12)
  input_validation_policy: true
  backup_restore_safety: true
  silent_data_loss_fixed: true
  auto_schema_migration: true

  # Phase 11: WASM Parity & API Completeness (cost: 8)
  wasm_persistence_stubs_complete: true
  wasm_bindings_expanded: true
  framework_read_apis_added: true
  query_cache_zero_alloc: true

  # Phase 12: Ecosystem & Release Engineering (cost: 8)
  error_variants_refined: true
  persistence_health_check: true
  concurrent_access_tests: true
  backup_restore_tests: true

  # Phase 13: Documentation & DX Improvements (cost: 6)
  framework_config_documented: true
  singularity_config_documented: true
  readme_installation_section: true
  basic_in_memory_example: true
  cargo_aliases_created: true

  # Phase 14: Observability Completion (cost: 4)
  singularity_tracing_added: true
  cache_hit_miss_metrics: true
  reservoir_step_metrics: true

  # Phase 15: Testing Pragmatism (cost: 3)
  to_hypervector_benchmark_added: true
  critical_error_path_tests: true

  # Phase 16: WASM Parity (cost: 3)
  wasm_process_sequence_exposed: true
  wasm_memory_export_import: true

  # Phase 17: Comprehensive Testing (cost: 8) - Wave 8
  batch_operations_tests_added: true
  persistence_crud_tests_added: true
  export_import_tests_added: true
  cache_behavior_tests_added: true

  # Phase 18: Benchmark Coverage (cost: 6) - Wave 8
  persistence_crud_benchmarks_added: true
  hvec_bundle_benchmarks_added: true
  benchmark_coverage_percent: 45  # Up from 17%

  # Phase 19: 2026 GitHub Standards (cost: 4) - Wave 8
  readme_badges_added: true
  pr_template_created: true
  issue_templates_created: true
  llms_full_txt_created: true

  # Phase 20: CLI Crate (cost: 12)
  cli_crate_created: true
  cli_commands_implemented: true
  cli_tests_passing: true
  shell_completions_generated: true

  # Phase 21: Production Hardening (cost: 18) - Wave 10
  async_lock_safety: true               # ADR-0031: RwLock held across await points
  cli_json_escaping: true               # ADR-0032: CLI JSON output safety
  cli_exit_code_correctness: true       # ADR-0032: CLI exit codes collapse to 255
  cli_error_output_format: true         # ADR-0032: --output-format ignored on errors
  cli_unused_config_flag: true          # ADR-0032: --config flag declared but unused
  wasm_panic_safety: true               # ADR-0033: Reflect::set().unwrap() in wasm.rs
  framework_metadata_injection: true    # ADR-0034: inject_concept_with_metadata API
  framework_builder_input_size: true    # ADR-0034: with_reservoir_input_size() missing
  wasm_batch_api_parity: true           # ADR-0034: WASM missing batch APIs
  cache_memory_guardrails: true         # ADR-0035: Query cache memory blow-up risk

  # Phase 22: CI/DX Hardening (cost: 10) - Wave 10
  loc_gate_recursive: true              # ADR-0036: LOC gate misses src/cli/ and src/bin/
  pre_commit_hook_installed: true       # ADR-0036: No pre-commit hook exists
  clippy_flags_consistent: true         # ADR-0036: CI vs local clippy flag mismatch
  post_commit_hook_fixed: true          # ADR-0036: post-commit runs cargo test + amends
  exitcode_crate_removed: true          # ADR-0036: Completed via ADR-0038
  cli_deps_gated: true                  # ADR-0036: Completed via ADR-0038

  # Phase 23: Rust Best Practices (cost: 6) - Wave 10
  must_use_annotations: true            # ADR-0037: Missing #[must_use] on constructors
  unsafe_docs_complete: true            # ADR-0037: SIMD unsafe blocks need better docs
  clippy_suppressions_targeted: true    # ADR-0037: File-wide #![allow] -> per-loop
  cli_json_serde: true                  # ADR-0037: Replace format! JSON with serde_json
  probe_unwrap_removed: true            # ADR-0037: serde_json::to_string().unwrap()

  # Phase 24: Cargo.toml Modernization (cost: 4) - Wave 10
  cargo_toml_modernized: true           # ADR-0038: Edition 2024, metadata, deps
  crates_io_ready: true                 # ADR-0038: Required metadata for publishing
  edition_2024: true                    # ADR-0038: Rust edition 2024 upgrade

  # GOAP/Plan Consistency
  actions_md_phase20_synced: true       # ACTIONS.md Phase 20 now correctly marked complete

  # Phase 25: Release Engineering (cost: 12) - Wave 11
  release_management_skill_created: true
  release_adr_created: true             # ADR-0039
  github_pages_workflow_created: true
  crates_io_trusted_publishing: true
  npm_provenance_publishing: true
  mdbook_docs_structure: true
  wave_11_complete: true

  # Phase 26: Release Automation & v0.1.0 (cost: 16) - Wave 12
  release_workflow_fixed: true               # ADR-0042: Fixed release.yml workflow
  release_scripts_non_interactive: true      # ADR-0042: release-manager.sh has --yes flag
  release_validate_dry_run: true             # ADR-0042: cargo publish --dry-run in release-manager.sh
  release_changelog_automation: true         # ADR-0042: release-manager.sh handles CHANGELOG
  release_version_sync: true                 # ADR-0042: release-manager.sh syncs all doc versions
  release_workspace_clean: true              # ADR-0042: Updated .gitignore
  release_notify_error_handling: true        # ADR-0042: release.yml checks job results
  release_manager_script: true               # ADR-0042: scripts/release-manager.sh created
  v010_tag_created: true                     # ADR-0042: v0.1.0 tag pushed
  v010_published: true                       # ADR-0042: Published to crates.io
  v011_tag_created: true                     # v0.1.1 tag pushed
  v011_published: true                       # Published to crates.io 2026-02-27

  # Phase 26B: npm Publishing Fix (cost: 7) - Wave 12
  # Per ADR-0046: npm requires Node.js 24 for OIDC, or use NPM_TOKEN
  npm_publish_wasm_opt_fixed: true           # Fix: --enable-bulk-memory for wasm-opt
  npm_publish_workflow_updated: true         # ADR-0046: Updated to Node.js 24 + token fallback
  npm_pkg_json_repository: true              # repository field + publishConfig.provenance
  npm_first_publish_manual: true             # v0.1.0 exists on npm (verified via Snyk + API)
  npm_package_exists: true                   # @d-o-hub/chaotic_semantic_memory at v0.1.0
  npm_token_expired: true                     # NPM_TOKEN secret is expired/revoked
  npm_oidc_configured: true                   # Verified: v0.3.2 published via OIDC (2026-04-10)
  npm_publish_automated: true                 # Verified: npm workflow dispatch succeeded
  npm_node24_required: true                   # Node.js 24 ships npm v11 required for OIDC

  # Post-1.0 Deferred Work (ADR-0024, ADR-0025, ADR-0026)
  # Per Swarm Consensus 2026-02-17: Advanced features deferred until user demand
  deferred_concept_ttl: false           # ADR-0024: advanced TTL policy automation (baseline TTL shipped)
  deferred_association_decay: false     # ADR-0025: Weighted forgetting
  deferred_namespace_isolation: false   # ADR-0026: Multi-tenancy
  deferred_phase2_optimizations: false  # ADR-0024: SIMD completion, PQ, LSH
  deferred_trigger_threshold: "User demand or >200k concepts with latency issues"

  # Phase 27: Post-Release Security & Hardening (cost: 38) - Wave 13
  # Triggered by: Specialist analysis swarm findings 2026-02-20
  # Implementation: ADR-0047 - Security & Performance Hardening for v0.2.0
  swarm_analysis_completed: true
  swarm_analysis_findings_total: 9
  
  # Error handling improvements
  error_source_attributes_added: true          # 2026-04-29: verified #[source] on Database/Reservoir variants in src/error.rs:10,29
  production_expect_fixed: true               # Fix expect() in framework.rs:177 ✅
  error_context_enhanced: false                # Add remediation hints to errors (deferred)
  
  # Security hardening
  bincode_size_limits_added: true             # CRITICAL: Add 100MB limit to deserialization ✅
  path_traversal_protection_added: true       # Validate file paths in framework_ops ✅
  metadata_validation_added: true              # Check metadata size before serialization (done in ADR-0034)
  security_tests_added: false                  # Property-based security tests (deferred)
  
  # Performance optimizations
  bundle_allocation_optimized: true            # Fold pattern already efficient ✅
  find_similar_optimized: true                 # Add find_similar_arc, optimize computation ✅
  to_hypervector_parallelized: true            # Rayon parallelization on 80-word loop ✅
  cache_rwlock_fixed: true                    # Replace Mutex with RwLock ✅
  avx2_simd_added: true                        # 2026-04-29: verified AVX2 + NEON paths in src/hyperdim_simd.rs (target_feature=avx2 line 73; aarch64 NEON line 102)
  
  # Memory leak prevention
  max_concepts_limit_added: true               # Hard ceiling (default 100K) ✅
  max_associations_limit_added: true           # Per-concept association limit ✅
  version_retention_configurable: true         # Version history configurable ✅
  version_retention_hard_ceiling: true         # Hard limit on version history ✅
  
  # Observability improvements
  reservoir_tracing_added: true                # Instrument hot path ✅
  persistence_tracing_added: true              # Instrument DB operations ✅
  cli_tracing_added: true                      # Instrument command entry points ✅
  tracing_coverage_percent: 70                 # Current: 70%, Target: 70% ✅
  error_log_correlation_fixed: true           # #[instrument(err)] on fallible fns ✅

  # Phase 28: Release Protocol & v0.2.0 Preparation (cost: 8)
  # ADR-0049: Release checklist and version sync protocol
  release_checklist_adr_created: true         # ADR-0049: Release checklist ✅
  release_checklist_document: true           # Document all version reference locations ✅
  v020_release_planned: false                # Plan v0.2.0 release with checklist
  version_sync_script_created: true          # scripts/sync-version.sh automation ✅
  pre_release_validate_script_created: true  # scripts/pre-release-validate.sh ✅ ADR-0052

  # Phase 29: Real-World Readiness Bug Fixes (cost: 6) - Wave 14
  # ADR-0051: Real-World Readiness & Quality Hardening
  cached_top_k_propagation_fixed: true       # Fix max_cached_top_k not propagated ✅
  cached_top_k_default_aligned: true        # Align default to 100 ✅
  not_found_error_variant_added: true        # New NotFound variant in error.rs ✅
  json_import_size_limit_added: true         # Cap JSON import at 100MB ✅

  # Phase 30: Real-Life Usage Examples (cost: 8) - Wave 14
  chatbot_memory_example_added: true        # examples/chatbot_memory.rs ✅
  document_rag_example_added: true           # examples/document_rag.rs ✅
  knowledge_graph_example_added: true       # examples/knowledge_graph.rs ✅
  streaming_temporal_example_added: true     # examples/streaming_temporal.rs ✅

  # Phase 31: Edge Case Tests (cost: 5) - Wave 14
  builder_config_tests_added: true          # tests/builder_config.rs ✅
  import_adversarial_tests_added: true      # tests/import_adversarial.rs ✅
  eviction_cache_tests_added: true          # tests/eviction_cache.rs ✅

  # Wave 14: Real-World Readiness
  wave_14_name: "Real-World Readiness"
  wave_14_started_at: "2026-02-28"
  wave_14_focus: "Phase 29-31: Bug fixes, real-world examples, edge case tests"
  wave_14_complete: true

  # Wave 15: API Hardening & New Features (ADR-0053, ADR-0054)
  wave_15_name: "API Hardening & New Features"
  wave_15_started_at: "2026-03-02"
  wave_15_focus: "Phase 32-40: Safety fixes, API completeness, error hardening, docs, WASM parity, text encoding, metadata filtering, graph traversal, bundle accumulator"
  wave_15_complete: true

  # Phase 32: Production Safety (cost: 6) - Wave 15 ✅ COMPLETE
  # ADR-0053: API Hardening & New Features
  reservoir_try_into_unwrap_removed: true    # reservoir.rs:323 try_into().unwrap() ✅
  persistence_semaphore_deadlock_fixed: true  # Nested acquire_remote_slot in init_schema→apply_migrations→schema_version ✅
  version_row_get_unwrap_fixed: true          # persistence.rs:464 row.get().unwrap_or(0) ✅
  validate_path_current_dir_fixed: true       # framework_ops.rs:50 current_dir().unwrap_or_default() ✅

  # Phase 33: API Completeness (cost: 10) - Wave 15 ✅ COMPLETE
  framework_update_concept_vector: true       # update_concept_vector(id, vector) on framework ✅
  framework_update_concept_metadata: true     # update_concept_metadata(id, metadata) on framework ✅
  framework_disassociate: true                # disassociate(from, to) on framework + persistence ✅
  framework_clear_associations: true          # clear_associations(from) on framework + persistence ✅
  singularity_bundle_strict: true             # bundle_concepts_strict(ids) returns NotFound ✅
  singularity_clear_cache: true               # clear_similarity_cache() public API ✅
  builder_version_retention: true             # 2026-04-29: verified with_version_retention(n) at src/framework_builder.rs:163

  # Phase 34: Error Handling Hardening (cost: 4) - Wave 15 ✅ COMPLETE
  error_source_chain_support: true            # 2026-04-29: verified #[source] on Database/Reservoir in src/error.rs:10,29
  stats_db_size_optional: true                # FrameworkStats db_size_bytes: Option<u64> ✅
  dead_dimension_check_removed: true          # Singularity::inject redundant data.len() check ✅

  # Phase 35: Documentation Pass (cost: 4) - Wave 15 ✅ COMPLETE
  reservoir_invariants_documented: true        # input_size, stride, spectral radius docs ✅
  persistence_schema_documented: true          # version retention, migration semantics ✅
  load_merge_behavior_documented: true         # load_replace vs load_merge ✅
  wasm_parity_notes_added: true               # lib.rs WASM parity docs ✅

  # Phase 36: WASM API Parity (cost: 4) - Wave 15 ✅ COMPLETE
  wasm_update_concept_exposed: true           # update_concept in WASM ✅
  wasm_disassociate_exposed: true             # disassociate in WASM ✅
  wasm_stats_exposed: true                    # concept_count/stats in WASM ✅
  wasm_persistence_story_documented: true     # WASM → bytes → IndexedDB docs ✅

  # Phase 37: Text-to-Hypervector Encoding (cost: 8) - Wave 15 ✅ COMPLETE
  # ADR-0054: High-Impact New Features
  text_encoder_created: true                   # src/encoder.rs with TextEncoder ✅
  text_encoder_deterministic: true             # Stable hash → seeded PRNG → HVec10240 ✅
  text_encoder_position_aware: true            # Positional permutation encoding ✅
  text_encoder_ngram_support: true             # Optional character n-gram overlay ✅
  framework_inject_text: true                  # inject_text(id, text) convenience ✅
  framework_probe_text: true                   # probe_text(query, top_k) convenience ✅
  text_encoder_wasm_compatible: true           # Works in WASM target ✅

  # Phase 38: Metadata-Filtered Similarity Search (cost: 6) - Wave 15 ✅ COMPLETE
  # ADR-0054: High-Impact New Features
  metadata_filter_types_created: true          # MetadataFilter enum (Eq, In, Exists, And, Or, Not) ✅
  singularity_find_similar_filtered: true      # find_similar_filtered(query, top_k, filter) ✅
  framework_probe_filtered: true              # 2026-04-29: verified at src/framework.rs:199
  metadata_filter_wasm_exposed: true          # 2026-04-29: verified at src/wasm_ext.rs:129 (probe_filtered)

  # Phase 39: Association Graph Traversal (cost: 8) - Wave 15 ✅ COMPLETE
  # ADR-0054: High-Impact New Features
  singularity_neighbors: true                  # neighbors(id, min_strength) API ✅
  singularity_bfs: true                        # bfs(start, config) API ✅
  singularity_shortest_path: true              # shortest_path(from, to, config) API ✅
  singularity_incoming_associations: true      # incoming_associations(id) reverse lookup ✅
  framework_traverse: true                    # 2026-04-29: verified at src/framework.rs:220
  framework_shortest_path: true               # 2026-04-29: verified at src/framework.rs:233
  graph_traversal_wasm_exposed: true          # 2026-04-29: verified neighbors/bfs/shortest_path/traverse in src/wasm_ext.rs:73,92,113,159

  # Phase 40: Incremental Bundle Accumulator (cost: 4) - Wave 15 ✅ COMPLETE
  # ADR-0054: High-Impact New Features
  bundle_accumulator_created: true             # BundleAccumulator struct ✅
  bundle_accumulator_add_remove: true          # add/remove/finalize API ✅
  bundle_accumulator_streaming: true           # Sliding-window memory support ✅

  # Phase 41: Memory Change Events (cost: 4) - Wave 15 (DEFERRED)
  # ADR-0054: High-Impact New Features
  memory_event_enum_created: true             # 2026-04-29: verified MemoryEvent in src/framework_events.rs
  framework_subscribe: true                   # 2026-04-29: verified subscribe() at src/framework_events.rs:42
  memory_events_wasm_compatible: true         # 2026-04-29: emitter wired (framework.rs:127,162,282,304)

  # ═══════════════════════════════════════════════════════
  # Wave 16: Production Polish & Correctness (ADR-0055)
  # ═══════════════════════════════════════════════════════
  wave_16_name: "Production Polish & Correctness"
  wave_16_started_at: "2026-03-04"
  wave_16_focus: "Phase 42-47: Panic elimination, correctness fixes, WASM parity, tests, benchmarks, docs"
  wave_16_complete: true
  wave_16_completed_at: "2026-03-04"

  # Phase 42: Panic Path Elimination (cost: 3)
  bundle_accumulator_try_remove: true         # try_remove() -> Result<()> + no-op remove
  reservoir_to_hvec_panic_removed: true        # Replace panic with MemoryError::Reservoir
  encoder_bundle_fallback_documented: true     # Document intentional zero fallback

  # Phase 43: Correctness Fixes (cost: 5)
  text_encoder_fnv1a_hash: true               # Replace DefaultHasher with actual FNV-1a
  text_encoder_hash_configurable: true        # TextEncoderConfig::hash_algorithm field
  text_encoder_golden_vectors: true           # Regression test with known input → known output
  shortest_path_weighted_dijkstra: true       # Implement actual weighted Dijkstra
  shortest_path_hops_preserved: true          # Rename current BFS to shortest_path_hops

  # Phase 44: WASM Parity Completion (cost: 4)
  wasm_update_metadata_exposed: true          # update_concept_metadata in WASM
  wasm_clear_associations_exposed: true       # clear_associations in WASM
  wasm_graph_traversal_exposed: true          # neighbors, bfs, shortest_path in WASM
  wasm_metadata_json_fidelity: true            # Fix lossy metadata via js_sys::JSON::parse

  # Phase 45: Test Coverage for Wave 15 Features (cost: 4)
  text_encoder_regression_tests: true         # Golden vector regression tests
  graph_traversal_cycle_tests: true           # Cycle detection, disconnected graphs
  bundle_accumulator_edge_tests: true         # Remove from empty (no panic), overflow
  filtered_search_edge_tests: true            # Empty/no-match/large dataset filters

  # Phase 46: Benchmark Coverage (cost: 3)
  text_encoder_benchmarks: true               # encode short/medium/long text
  filtered_search_benchmarks: true            # 100/1k/10k with filters
  graph_traversal_benchmarks: true            # BFS/shortest_path sparse/dense
  bundle_accumulator_benchmarks: true         # add/remove/finalize cycle

  # Phase 47: Documentation Refresh (cost: 2)
  changelog_v020_updated: true                # CHANGELOG.md for v0.2.0
  readme_encoder_graph_examples: true         # README.md with new feature examples
  book_encoder_graph_chapters: true            # book/src/ chapters for encoder + graph
  llms_txt_refreshed: true                    # Updated llms.txt and llms-full.txt

  # Phase 48: Performance Follow-up (cost: 7) - Completed 2026-03-09
  probe_scan_materialization_removed: true    # ADR-0056: find_similar_cached no longer clones all concepts on cache misses
  local_sqlite_wal_enabled: true              # ADR-0056: local SQLite init enforces journal_mode=WAL
  probe_scale_trigger_exceeded: true          # ADR-0056: activate optional ANN/LSH after >200k concepts with measured latency regression

  # Phase 54: Retrieval Optimization (cost: 15) - Wave 19 ✅ COMPLETE
  retrieval_hot_path_optimized: true
  reduced_candidate_retrieval_implemented: true
  benchmark_methodology_improved: true

  # ═══════════════════════════════════════════════════════
  # 2026-04-30 Gap Analysis — Wave 21-24 Roadmap
  # See: plans/GAP_ANALYSIS_2026_04_30.md
  # ADRs proposed: plans/adr/0066-0076.md
  # ═══════════════════════════════════════════════════════
  gap_analysis_2026_04_30_completed: true
  gap_analysis_2026_04_30_findings: 10
  gap_analysis_2026_04_30_adrs_drafted: 11
  gap_analysis_2026_04_30_total_cost: 116
  action_last_completed: gap_analysis_2026_04_30

  # Wave 21 P0 — Adoption Unblockers (queued, total cost 34)
  cli_framework_parity_complete: false        # ADR-0066 — 11 CLI subcommands missing
  mcp_server_implemented: false               # ADR-0067 — `csm mcp serve` for LLM agents
  adr_backfill_complete: false                # ADR-0076 — ~29 missing ADR files

  # Wave 22 P1 — Capability Ceiling (queued, total cost 40)
  hnsw_ann_index_implemented: false           # ADR-0068 — scale beyond 200k concepts
  probe_scale_ceiling_lifted: false           # ADR-0068 — current ceiling ~200k
  embedding_model_bridge_implemented: false   # ADR-0069 — fastembed/openai/voyage
  graphrag_retrieval_implemented: false       # ADR-0070 — anchor + BFS + joint score

  # Wave 23 P2 — Production Polish (queued, total cost 28)
  reranking_pipeline_implemented: false       # ADR-0071 — MMR + recency + cross-encoder
  otlp_exporter_implemented: false            # ADR-0072 — opentelemetry + prometheus
  namespace_isolation_implemented: false      # ADR-0073 — supersedes deferred ADR-0026
  version_history_surface_implemented: false  # ADR-0074 — activate dormant version table

  # Wave 24 P3 — Future Scale (queued, cost 14, depends on Wave 22)
  quantized_binary_hypervectors_implemented: false  # ADR-0075 — 32× memory compression

  # ═══════════════════════════════════════════════════════
  # Real-usage Verification + Clippy Audit (2026-04-30)
  # See: plans/VERIFICATION_2026_04_30.md
  # ═══════════════════════════════════════════════════════
  verification_2026_04_30_completed: true
  verification_2026_04_30_lifecycle_pass: true                # save/probe/export/import roundtrip OK
  verification_2026_04_30_benchmarks_pass: true               # bm25 + benchmark + persistence
  verification_2026_04_30_dist_channels_aligned: true         # crates.io + npm CLI + npm WASM all 0.3.5
  verification_2026_04_30_clippy_audit: true
  verification_2026_04_30_bm25_search_1000_us: 47.1           # was 64.4 µs (PR #129); now 47.1 µs (-27%)
  verification_2026_04_30_singularity_probe_50000_ms: 3.73    # < 10 ms target
  verification_2026_04_30_persistence_cold_start_us: 700      # ~705 µs
  verification_2026_04_30_archive_phase_skipped: true         # no native CLI command — tracked in ADR-0066
  verification_2026_04_30_delete_phase_skipped: true          # no native CLI command — tracked in ADR-0066

  # Clippy pedantic / nursery audit
  clippy_pedantic_surface_warnings: 936
  clippy_actionable_warnings: 110                             # float_cmp + drop_tightening + cast_* + const_fn + redundant_clone
  adr_0077_clippy_promotion_drafted: true                     # plans/adr/0077-clippy-pedantic-selective-promotion.md
  clippy_pedantic_promotion_complete: false                   # 5 themed PRs queued
  action_last_completed: verification_and_clippy_audit_2026_04_30
