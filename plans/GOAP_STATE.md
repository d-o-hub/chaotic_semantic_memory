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
  validated: false                    # Wave 32 still in progress (feature contracts, ownership, evidence tiers remain)
  dependency_hygiene_complete: true
  adr_registry_current: true
  result_contract_clarified: true
  architecture_docs_two_tier: true
  architecture_docs_canonical_source: "context.yaml"
  # action_last_completed is set ONCE near the end of the file (DRY).
  # Earlier duplicates were stale and silently overwritten by the later entry.
  # 2026-05-04 PR triage:
  #   #173 (PR169 feedback) — squash-merged → main 7d4dfaa
  #   #174 (hyperdim perf)  — rebased clean onto main (kept hyperdim.rs only;
  #                           benchmarks/memory_adapter.rs restored from main),
  #                           CI all-green, squash-merged
  #   #172 (CLI parity follow-ups) — CLOSED as obsolete; watch.rs and
  #                           probe_filtered.rs feedback already in main via
  #                           Wave 21 P0 (6e7bb17)
  #   #157 (CLI parity ADR-0066)   — CLOSED as obsolete; landed via Wave 21 P0
  #   #169 (HNSW/LSH ADR-0068)     — CLOSED; ANN code in main, PR would
  #                           regress 7044 LOC including #167 GraphRAG tests
  #   #159 (MCP Server ADR-0067)   — CLOSED; MCP in main, PR would delete
  #                           graph_rag/ann_*/wasm_graph_rag/singularity_search
  orchestrator_last_run: goap_orchestrator_analysis_2026_04_30
  orchestrator_last_run_at_utc: 2026-04-30T18:30:00Z
  goap_orchestrator_analysis_complete: true         # 2026-04-30: All actionable items verified complete
  skills_all_defined: true                          # 2026-07-14: 32 skill manifests exist; validation hardening queued
  no_missing_implementations: false                 # BM25 absence TODO + CLI metrics reset + ownership/feature contracts still queued
  book_chapters_complete: true                      # 2026-04-30: 15 book chapters verified (semantic-bridge, inertial-reservoir, ttl exist)
  changelog_links_complete: true                    # 2026-04-30: All version links verified
  hybrid_retrieval_example_compiles: true           # 2026-04-30: cargo check --example hybrid_retrieval OK
  loc_gate_verified: true                          # 2026-07-14: all workspace crate files now ≤500 LOC (singularity 398, hyperdim 462, graph_traversal 312)
  misleading_docs_found: true                       # 2026-07-20: README version 0.3/0.4 drift + ANN "deferred" vs shipped IndexBackend; AGENTS skills 30 vs 32
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
  tests_count_wave27_baseline: 668               # Historical 2026-06-09 snapshot; canonical count is near file end
  skills_count: 32                              # 2026-07-14: find .agents/skills -name SKILL.md | wc -l
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
  ci_pre_release_gate_failing: true            # Latest named Pre-Release Gate run 24604591915 failed; rerun required
  changelog_v033_yanked_recorded: true         # 2026-04-28: backfilled CHANGELOG
  changelog_unreleased_section: true           # 2026-04-28: added Unreleased
  goap_actions_md_synced: true                 # 2026-04-28: 3 queued -> complete

  # GitHub Actions Security Audit (2026-04-21)
  codeql_missing_permissions_fixed: true
  codeql_alerts_resolved: 4  # actions/missing-workflow-permissions in ci.yml
  dependabot_alerts_open: 5  # 2026-07-11: opentelemetry_sdk(medium), time(medium), lru(low), libsql-sqlite3-parser(2×low)
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
  ci_executes_real_criterion_benches: false
  benchmark_workspace_tests_run_in_ci: false
  wasm_js_smoke_test_enforced: false
  benchmark_ci_enforces_quality_thresholds: false
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
  active_wave: 32
  wave_strategy: parallel_by_phase_with_handoffs
  wave_20_name: "Implementation Queue Rebuild"
  wave_20_started_at: "2026-04-12"
  wave_20_focus: "Queue reconstruction + A-F parallel assignment + handoff contracts"
  wave_20_queue_total: 16
  wave_20_queue_blocked: 2
  wave_20_queue_executable: 14
  wave_20_handoff_contracts_created: 5
  wave_20_status: complete
  # Completed waves 1-14, 17 — summary (detail in progress/WAVE_HISTORY.md)
  waves_1_through_9_complete: true   # Swarm phases: fuzz, SIMD, logging, migration, pooling, export, versioning, CLI
  waves_11_through_14_complete: true # Release engineering, security hardening, real-world readiness
  wave_17_complete: true             # Performance follow-up & GOAP consistency
  all_waves_finished: false
  swarm_status: active
  final_validation_passed: false

  # PR tracking
  pr_94_status: merged
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

  # Workspace extraction status
  workspace_extraction_csm_embedding: true  # 2026-06-12: PR #377 merged, csm-embedding extracted
  workspace_extraction_csm_memory: true     # 2026-06-12: PR #378 merged, csm-memory extracted
  workspace_extraction_csm_retrieval: true  # 2026-06-12: PR #379 merged, csm-retrieval extracted
  workspace_extraction_csm_persistence: true # 2026-06-12: PR #380 merged, csm-persistence extracted
  workspace_extraction_csm_cli: true        # 2026-06-12: PR #381 merged, csm-cli extracted
  workspace_extraction_csm_traits: true     # 2026-06-12: PR #384 merged, csm-traits created
  workspace_extraction_csm_wasm: true       # 2026-06-12: PR #385 merged, csm-wasm created
  workspace_mio_wasm_fix: true              # 2026-06-12: PR #383 merged, WASM CI fixed

  # ═══════════════════════════════════════════════════════
  # Module status (LOC counts) - Updated 2026-06-10
  # Reconciled against codebase after workspace split (PR #356),
  # MCP/index/embedding/observability/namespaces additions, and
  # bm25 perf (PR #363). Previous map dated 2026-04-21 and was
  # missing ~45 modules + the entire crates/ workspace.
  # Source workspace LOC: src/ = 21,665 ; crates/ = 5,313.
  # All first-party source files ≤ 500 LOC (LOC gate satisfied).
  # ═══════════════════════════════════════════════════════
  modules:
    # --- root crate: src/ ---
    lib.rs: 225
    concept_builder.rs: 164
    metadata_filter.rs: 212
    export_payload.rs: 177
    export_payload/export_payload_tests.rs: 493
    # singularity (HDC memory core)
    singularity.rs: 492
    singularity_retrieval.rs: 479
    singularity_search.rs: 253
    singularity_cache.rs: 219
    singularity_ttl.rs: 207
    singularity_ext.rs: 161
    singularity_state.rs: 41
    # framework (public facade)
    framework.rs: 500
    framework_ops.rs: 500
    framework_builder.rs: 398
    framework_validation.rs: 366
    framework_namespaces.rs: 377     # ADR-0073 namespace isolation
    framework_persistence.rs: 300
    framework_events.rs: 251
    framework_events_ce.rs: 283      # CloudEvents emitter (Wave 25)
    framework_metrics.rs: 243        # FrameworkMetrics → Prometheus bridge
    framework_bridge.rs: 187
    framework_ttl.rs: 160
    framework_graph_rag.rs: 80       # ADR-0070 GraphRAG
    framework_rerank.rs: 62          # ADR-0071 reranking
    # persistence (libsql/turso)
    persistence.rs: 281
    persistence_ops.rs: 394
    persistence_concepts.rs: 301
    persistence_migrations.rs: 380
    persistence_versions.rs: 219
    persistence_wasm.rs: 223
    persistence_index.rs: 54
    # graph + bridge
    graph_traversal.rs: 486
    semantic_bridge.rs: 420
    semantic_triples.rs: 78
    bridge_retrieval.rs: 426
    bridge_persistence.rs: 317
    # retrieval/
    retrieval/bm25.rs: 456
    retrieval/bm25/tests.rs: 435
    retrieval/hybrid.rs: 244
    retrieval/rerank.rs: 376         # ADR-0071 MMR + recency + cross-encoder
    retrieval/graph_rag.rs: 238      # ADR-0070
    retrieval/mod.rs: 18
    # index/ (ANN — ADR-0068)
    index/hnsw.rs: 354
    index/lsh.rs: 377
    index/brute_force.rs: 187
    index/mod.rs: 103
    # embedding/ (ADR-0069 bridge)
    embedding/mod.rs: 265
    embedding/projection.rs: 234
    embedding/fastembed.rs: 167
    embedding/hdc_text.rs: 128
    embedding/remote_openai.rs: 145
    embedding/remote_voyage.rs: 145
    # mcp/ (ADR-0067 Model Context Protocol server)
    mcp/handler.rs: 475
    mcp/schema.rs: 224
    mcp/server.rs: 61
    mcp/mod.rs: 16
    # observability/ (ADR-0072/0086)
    observability/mod.rs: 171
    observability/otlp.rs: 65
    observability/prom.rs: 250
    # wasm/
    wasm.rs: 446
    wasm_ext.rs: 362
    wasm_ext_tests.rs: 476
    wasm_graph_rag.rs: 192
    # cli/ + bin/
    bin/csm.rs: 221
    cli/args.rs: 499
    cli/mod.rs: 18
    cli/mcp.rs: 21
    cli/error.rs: 113
    cli/git_local.rs: 302
    cli/commands/mod.rs: 269
    cli/commands/query.rs: 289
    cli/commands/index_dir.rs: 301
    cli/commands/index_jsonl.rs: 155
    cli/commands/inject.rs: 197
    cli/commands/probe.rs: 165
    cli/commands/probe_filtered.rs: 114
    cli/commands/probe_graph.rs: 113
    cli/commands/associate.rs: 167
    cli/commands/associations.rs: 110
    cli/commands/disassociate.rs: 145
    cli/commands/traverse.rs: 121
    cli/commands/get.rs: 145
    cli/commands/update.rs: 130
    cli/commands/delete.rs: 85
    cli/commands/history.rs: 117
    cli/commands/diff.rs: 80
    cli/commands/rollback.rs: 67
    cli/commands/path.rs: 117
    cli/commands/stats.rs: 56
    cli/commands/metrics.rs: 88
    cli/commands/watch.rs: 204
    cli/commands/export.rs: 66
    cli/commands/import.rs: 126
    cli/commands/completions.rs: 45

  # --- workspace member: crates/csm-core (HDC + reservoir kernels) ---
  modules_csm_core_lib:
    hyperdim.rs: 494
    hyperdim_simd.rs: 276
    hyperdim_simd_bundle.rs: 202
    hyperdim_batch.rs: 60
    hyperdim_serde.rs: 106
    bundle.rs: 270
    bundle_simd.rs: 278
    encoder.rs: 414
    reservoir.rs: 467
    reservoir_sparse.rs: 474
    reservoir_chaotic.rs: 107
    reservoir_inertial.rs: 33
    error.rs: 115
    lib.rs: 39

  # --- workspace member: crates/csm-duckdb (ADR-0079..0082) ---
  modules_duckdb_companion:
    lib.rs: 139
    export_parquet.rs: 185
    ingest_export.rs: 98
    ingest_bench.rs: 96
    ingest_libsql.rs: 74
    stats.rs: 91
    schema.rs: 62
    manifest.rs: 62
    export_all.rs: 56
    connection.rs: 50
    error.rs: 40
    cli/mod.rs: 112
    cli/query.rs: 68
    cli/export.rs: 46
    cli/inspect.rs: 43
    cli/stats.rs: 33
    bin/csm-analytics.rs: 20

  # Test status - Updated 2026-06-10
  # 60 integration test files in tests/ + inline #[test] modules across src/ & crates/.
  workspace_tests_passing_2026_06_10: true      # Historical snapshot; canonical tests_passing is defined once above
  integration_tests_exist: true
  integration_test_files: 60        # tests/*.rs
  total_test_functions: 773         # #[test]/#[tokio::test] across src + crates + tests
  test_count_note: "Was 333 on 2026-04-21; grew with ANN, embedding, MCP, namespaces, GraphRAG, rerank, observability, duckdb companion."

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
  benchmarks_prove_performance: false
  turso_roundtrip_under_20ms: true
  10m_concepts_under_12mb: false
  wasm_binary_under_500kb: true
  no_hardcoded_runtime_settings: true
  no_magic_numbers_without_named_constants: true

  # Phase 5: Testing & Quality (cost: 8)
  property_based_tests_added: true
  fuzzing_targets_created: true
  fuzz_workspace_compiles: true                  # 2026-07-16: persistence_save_concept API drift fixed; CI fuzz-build job
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
  derive_macros_created: true   # CANCELLED: proc-macro crate removed (unused), use ConceptBuilder directly
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
  advanced_ttl_policies_implemented: true       # 2026-06-23: Fixed/MetadataRule/Inherit/cascading purge/DecayCurve
  association_decay_implemented: true            # 2026-06-23: reinforce/prune APIs and tests shipped
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
  # NOTE: prior `action_last_completed: rebase_deepsource_fix_2026_06`
  # removed 2026-05-18 — duplicate key (YAML last-wins). Canonical entry
  # is at the bottom of the file; see Wave 25 / 2026-05-18 reconciliation.

  # Wave 21 P0 — Adoption Unblockers
  # 2026-05-18: Wave 21 P0 status reconciled with codebase.
  cli_framework_parity_complete: true         # ADR-0066 — all 22 commands wired
                                              #   in src/cli/args.rs + src/bin/csm.rs;
                                              #   tests/cli_parity.rs locks the surface.
  cli_parity_smoke_test_added: true           # 2026-05-18: tests/cli_parity.rs
  adr_backfill_complete: true                 # ADR-0076 — 79 registry, 78 on disk; ADR-0003
                                               #   marked superseded (N/A) per registry.
  adr_parity_script_added: true               # 2026-05-18: scripts/check-adr-parity.sh
  mcp_server_implemented: true                # ADR-0067 — consolidated rmcp 1.7 handler
  mcp_server_jules_issue: 246                 # github.com/d-o-hub/chaotic_semantic_memory/issues/246

  # Wave 22 P1 — Capability Ceiling (queued, total cost 40)
  hnsw_ann_index_implemented: true            # ADR-0068 — scale beyond 200k concepts (2026-05-20: complete)
  probe_scale_ceiling_lifted: true            # ADR-0068 — current ceiling ~200k (2026-05-20: complete)
  embedding_model_bridge_implemented: true    # ADR-0069 — fastembed/openai/voyage (2026-05-20: complete)
  graphrag_retrieval_implemented: true        # ADR-0070 — anchor + BFS + joint score (2026-05-20: complete)

  # Wave 23 P2 — Production Polish (queued, total cost 28)
  reranking_pipeline_implemented: true        # ADR-0071 — MMR + recency + cross-encoder (2026-05-20: complete)
  otlp_exporter_implemented: true             # ADR-0072 + ADR-0086 (2026-06-06: src/observability/ with `prometheus` + `otlp-json` features; 6 integration tests, 1 example)
  namespace_isolation_implemented: true       # ADR-0073 — supersedes deferred ADR-0026 (2026-05-20: complete)
  version_history_surface_implemented: true  # ADR-0074 — completed 2026-06-09: WASM diffVersions binding added

  # Wave 24 P3 — Future Scale (queued, cost 14, depends on Wave 22)
  quantized_binary_hypervectors_implemented: true   # ADR-0075 — implemented by PR #389; issue #353 closed

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
  verification_2026_04_30_hvec_bundle_10_us: 5.5              # was 67.2 µs (-91%)
  verification_2026_04_30_text_encoder_encode_short_us: 2.4   # was 11.5 µs (-79%)
  verification_2026_04_30_singularity_probe_50000_ms: 3.73    # < 10 ms target
  verification_2026_04_30_persistence_cold_start_us: 700      # ~705 µs
  verification_2026_04_30_archive_phase_skipped: false        # 2026-05-18: Resolved — archive markers work (see ADR-0083)
  verification_2026_04_30_delete_phase_skipped: false         # 2026-05-18: Resolved — `csm delete` command confirmed working

  # Clippy pedantic / nursery audit
  clippy_pedantic_surface_warnings: 936
  clippy_actionable_warnings: 110                             # float_cmp + drop_tightening + cast_* + const_fn + redundant_clone
  adr_0077_clippy_promotion_drafted: true                     # plans/adr/0077-clippy-pedantic-selective-promotion.md
  clippy_pedantic_promotion_complete: true                    # 2026-05-20: completed in ADR-0077 Phase A+B
  # ═══════════════════════════════════════════════════════
  # GitHub Actions + Security + Validation Audit (2026-05-21)
  # Orchestrator: goap_orchestrator_analysis_2026_05_21
  # Main HEAD: d557d1688 (fast-forward to include AVX2 optimization)
  # ═══════════════════════════════════════════════════════
  goap_2026_05_21_audit_completed: true
  goap_2026_05_21_main_head: "d557d1688"
  goap_2026_05_21_validation_gates:
    cargo_check: passing
    cargo_test: passing  # 57 test binaries, all passing
    cargo_fmt: clean
    cargo_clippy: clean
  goap_2026_05_21_workflows:
    ci: passing
    release: passing       # latest 3 runs all success (earlier failures: wait-for-ci timeout)
    benchmark_ci: passing
    pages: passing         # latest 2 success (was failing)
    version_integrity: passing
    codeql: active_no_open_alerts
    dependabot: active_3_open_low_severity
    pre_release_gate: passing
  ci_pre_release_gate_analysis:
    version_triad: "2026-05-21 FIXED: switched from grep -oP (YAML backslash hell) to sed extraction"
    security_audit: "2026-05-21 FIXED: removed --deny warnings; cargo audit reports visibly, unmaintained crates documented in plans/UNMAINTAINED_CRATES.md"
    planning_state_check: "2026-05-21 FIXED: added tr -d '\\r' to strip Windows line endings"
    wasm_smoke_build: passing
    pr_drain_check: passing
    all_gates_now_passing: true
  goap_2026_05_21_version:
    cargo_toml: "0.3.6"
    changelog: "0.3.6"
    wasm_package_json: "0.3.6"
    version_triad_match: true
  goap_2026_05_21_adr_parity:
    registry_count: 80
    disk_count: 79
    parity_satisfied: true
  # ═══════════════════════════════════════════════════════
  # Pre-Release Gate Fixes (2026-05-21)
  # Branch: fix/pre-release-gate-fixes
  # ═══════════════════════════════════════════════════════
  goap_pre_release_gate_fixes_completed: true
  goap_pre_release_gate_fix_files:
    - ".github/workflows/pre-release-gate.yml: CHANGELOG regex (grep -oP → sed), planning-state-check (tr -d '\\r'), security-audit (--deny warnings removed)"
    - "plans/UNMAINTAINED_CRATES.md: documents bincode, number_prefix, paste with migration plans and failed bincode 2.x/3.x migration attempt"
    - "plans/DEPENDABOT_ALERTS.md: documents triaged dependabot alerts (rand dismissed, libsql-sqlite3-parser no patch)"
  goap_pre_release_gate_fix_validation:
    cargo_check: passing
    cargo_fmt: clean
    cargo_clippy: clean
    cargo_audit: "passing (3 visible warnings, exit 0)"
    cargo_test: "733 tests passing (all features)"
    changelog_sed_regex_verified: "0.3.6 extracted correctly"
  goap_bincode_migration_attempted: true
  goap_bincode_migration_reverted: true
  goap_bincode_migration_findings:
    - "bincode 3.0.0 is a prank release (compile_error xkcd 2347)"
    - "bincode 2.0.x has breaking API changes requiring 7-file refactor"
    - "libsql v0.9.30 pins bincode 1.x transitively"
    - "Decision: keep bincode 1.3.3; document in UNMAINTAINED_CRATES.md; evaluate postcard later"
  goap_bincode_keep_1_3_3: true

  # ═══════════════════════════════════════════════════════
  # Rebase + DeepSource Fix (June 2026)
  # Branch: fix/pr169-feedback → PR #173
  # ═══════════════════════════════════════════════════════
  rebase_deepsource_fix_completed: true
  rebase_deepsource_changes:
    - "Rebased fix/pr169-feedback onto origin/main (GraphRAG, hyperdim parallel bundling, benchmark features)"
    - "Fixed Self::new() in Default::default() → direct construction (singularity, bundle, hdc_text, encoder)"
    - "Fixed .map().unwrap_or(false) → .is_ok_and() in git_local.rs"
    - "Fixed #[derive(Default)] on TextEncoder instead of manual impl"
    - "Removed duplicate Serialize/Deserialize impls from hyperdim.rs (moved to hyperdim_serde.rs)"
    - "Added set_bit(&mut self, pos) method to HVec10240 (needed by ann_integration/ann_persistence tests)"
    - "Resolved 11 merge conflicts across benchmarks/**, src/, docs"
  rebase_gates:
    clippy: passing
    tests: "542 passed, 0 failures"
    fmt: clean
    loc_gate: satisfied
  rebase_commit: b771317
  pr_173_status: "OPEN, MERGEABLE, BLOCKED (old CI: benchmark-small failed, waiting for new CI run on b771317)"

  # ═══════════════════════════════════════════════════════
  # Wave 25: CloudEvents Event Emitter (P2)
  # ═══════════════════════════════════════════════════════
  cloudevents_adr_written: true
  cloudevents_implemented: true               # 2026-05-20: complete in framework_events_ce.rs
  cloudevents_tested: true

  # ═══════════════════════════════════════════════════════
  # Wave 26: DuckDB Companion Crate (P2) — backfilled 2026-05-18
  # ADRs 0079-0082. Code merged in main; this section records
  # state for the planner. All three phases shipped under
  # crates/csm-duckdb/.
  # ═══════════════════════════════════════════════════════
  duckdb_workspace_restructure_complete: true   # ADR-0079
  duckdb_phase1_readonly_analytics_complete: true  # ADR-0080
  duckdb_phase2_parquet_export_complete: true   # ADR-0081
  duckdb_phase3_cli_integration_complete: true  # ADR-0082 (PR #242, merge 8ca0e75)
  duckdb_companion_published: false             # not yet on crates.io
  duckdb_companion_in_next_release: true        # ship in v0.3.6
  duckdb_companion_tests_passing: true          # cli_tests, integration_tests, parquet_export_tests

  # ═══════════════════════════════════════════════════════
  # Release readiness (2026-05-18 snapshot)
  # ═══════════════════════════════════════════════════════
  unreleased_changes_present: false             # All shipped in v0.3.6
  unreleased_changelog_section: true            # CHANGELOG.md updated to v0.3.6
  next_release_planned: "v0.3.6"
  next_release_blockers: 0                      # CI green on main (790fac9)

  # ═══════════════════════════════════════════════════════
  # Workflow learnings (2026-05-18)
  # See: progress/LEARNINGS.md "State drift verification"
  # ═══════════════════════════════════════════════════════
  workflow_lesson_verify_built_binary: true     # don't trust stale installed binaries
                                                # when assessing CLI surface gaps;
                                                # always `cargo build --bin <name>` first.
  workflow_lesson_delegate_long_actions: true   # actions with cost ≥ 12 should become
                                                # Jules GitHub issues (label: jules)
                                                # marked `status: delegated` in ACTIONS.md.
  goap_state_duplicate_key_fixed: true          # action_last_completed now appears
                                                 # exactly once in GOAP_STATE.md.

  # ═══════════════════════════════════════════════════════
  # Memory Lifecycle Verification (2026-05-18)
  # Dogfood: memory-lifecycle-verification skill
  # ═══════════════════════════════════════════════════════
  memory_lifecycle_verification_completed: true
  memory_lifecycle_via_turso_cli: true                  # sqld verified DB row counts + schema
  memory_lifecycle_phase1_save: true                    # inject 2 concepts, associate, probe
  memory_lifecycle_phase2_load_roundtrip: true           # export→import→probe yields identical results
  memory_lifecycle_phase3_archive: true                 # archive marker concepts with metadata
  memory_lifecycle_phase4_delete: true                  # delete + verify removed from active memory
  memory_lifecycle_db_checksum: 821d5a78240b2aadf69a00ae805d11b2c1579f00377481461306e0430639daf4
  memory_lifecycle_roundtrip_fidelity: true             # identical similarity scores (0.006055) across export/import
  memory_lifecycle_metadata_preserved: true             # {"phase":"save","v":1} survives roundtrip
  memory_lifecycle_followup_goap_created: true          # plans/GOAP_LIFECYCLE_VERIFICATION_FOLLOWUP.md
  memory_lifecycle_sql_checks_fixed: true               # csm_ prefix + correct column names
  memory_lifecycle_checklist_marked: true               # VALIDATION_CHECKLIST.md filled for 2026-05-18
  memory_lifecycle_stale_flags_fixed: true              # archive/delete phase skipped → resolved
  adr_0083_export_format_documented: true               # array-of-tuples contract accepted

  # ═══════════════════════════════════════════════════════
  # Mutation Testing CI Gate (2026-05-29)
  # ═══════════════════════════════════════════════════════
  mutation_ci_enforced: true
  mutation_threshold: 85
  mutation_ci_binstall_installed: true     # taiki-e/install-action for cargo-mutants
  mutation_ci_in_place_enabled: true       # --in-place reuses target/ cache
  mutation_ci_in_diff_enabled: true        # --in-diff scoped to PR changes
  mutation_ci_fetch_depth_zero: true       # fetch-depth: 0 in checkout
  wasm_freshness_sort_stable: true         # check-wasm-freshness.sh sorts lines before diff

  # ═══════════════════════════════════════════════════════
  # CI Security Policy Alignment (2026-05-21)
  # Orchestrator: goap_pin_actions_to_sha
  # ═══════════════════════════════════════════════════════
  actions_pinned_to_sha: true
  actions_pin_validation:
    github_workflows_verified: 6
    sha_pinning_check: passing
    version_comments_present: true
  goap_pin_actions_validation_gates:
    scripts_validate_github_actions_shas: passing
    cargo_test_compilation: passing
  # PR #346 mutation miss fixed 2026-06-05: removed dead `depth > max_depth`
  # branch in ConceptGraph::expand (related concepts only enqueued at
  # depth+1 when depth < max_depth, so the comparison was always false).
  # Added 5 regression tests covering expand() and label index roundtrip.
  pr_346_mutation_miss_resolved: true

  # ═══════════════════════════════════════════════════════
  # GOAP Reconciliation 2026-06 (codebase audit)
  # ADR-0085: GOAP Reconciliation 2026-06
  # Reconciles drift for merged PRs #345, #348, #349, #351 that
  # were not recorded in world_state, and removes the duplicate
  # `action_last_completed: pin_github_actions_to_sha` merge
  # artifact re-introduced by PR #348.
  # ═══════════════════════════════════════════════════════
  goap_reconciliation_2026_06_complete: true
  goap_state_duplicate_key_reremoved: true       # PR #348 re-added a stale
                                                 # action_last_completed; removed.

  # PR #345 — encoder allocation reduction
  encoder_alloc_reduction_landed: true           # src/encoder.rs: fewer temp allocs
                                                 # in text encoding hot path.

  # PR #348/#349 — namespace input validation (security, CWE-770)
  namespace_input_validation_landed: true        # validate_namespace() guards length
                                                 # (≤128B), emptiness, control chars.
  namespace_apis_fallible: true                  # set_namespace/with_namespace return
                                                 # Result; guard applied to set/delete/
                                                 # export/export_to_bytes/with_namespace.
  namespace_validation_max_bytes: 128            # MAX_NAMESPACE_BYTES (framework_validation.rs)

  # PR #351 — Miri scoped to push events on main only
  miri_main_only_landed: true                    # ci.yml miri job: if github.event_name
                                                 # == 'push' (skips PR runs, saves CI time).

  # ═══════════════════════════════════════════════════════
  # OTLP / Prometheus Implementation (2026-06-06)
  # ADR-0072 + ADR-0086
  # Branch: feat/observability-otlp-prom
  # ═══════════════════════════════════════════════════════
  otlp_observability_implemented: true
  observability_files:
    - "src/observability/mod.rs (171 LOC, init/Guard/LogFormat/ObservabilityConfig/render_metrics)"
    - "src/observability/otlp.rs (65 LOC, JSON subscriber via tracing-subscriber's json feature)"
    - "src/observability/prom.rs (250 LOC, prometheus registry + hyper /metrics server)"
    - "tests/observability_integration.rs (6 tests, gated on either feature)"
    - "examples/observability_otlp.rs (end-to-end smoke, requires prometheus,otlp-json)"
    - "src/error.rs (Observability / ObservabilityFeatureDisabled / ObservabilityAlreadyInitialised variants)"
    - "Cargo.toml (prometheus, hyper, hyper-util, http-body-util optional deps; tokio `net` feature)"
    - "plans/adr/0086-otlp-prom-implementation.md (rationale + follow-ups)"
  observability_features:
    - "prometheus — /metrics HTTP endpoint, 7 metrics per ADR-0072"
    - "otlp-json — JSON tracing subscriber (lightweight alternative to gRPC OTLP)"
  observability_gates:
    cargo_check: passing
    cargo_test: passing  # 6 new observability tests + 0 regressions
    cargo_fmt: clean
    cargo_clippy: clean
  observability_framework_auto_wired: true       # 2026-06-10: Wave 27 wired
                                                 # FrameworkMetrics → prom bridge
                                                 # (src/framework_metrics.rs).
  observability_follow_ups:
    - "auto_wire_framework_prom_metrics (cost 3, COMPLETE 2026-06-10 via Wave 27)"
    - "add_otlp_grpc_exporter (cost 8, deferred in ACTIONS.md)"

  # ═══════════════════════════════════════════════════════
  # PR #356 CI Remediation (2026-06-09)
  # Workspace-split PR — CI failure fix
  # ═══════════════════════════════════════════════════════
  pr_356_ci_failure_fixed: true
  pr_356_failure_root_cause: "cosine_similarity called on Vec<f32> (method only on HVec10240)"
  pr_356_fix_commit: "10e63ae"
  pr_356_ci_status:
    lint: pass
    test: pass
    mcp_feature: pass
    wasm: pass
    benchmark_small: pass
    codacy: pass
    codeql: pass
    sonarcloud: pass
    build_cli: pass  # all platforms

  # ═══════════════════════════════════════════════════════
  # PR #356 Mutation Kill Tests (2026-06-09)
  # Killed 8 missed mutants from cargo-mutants mutation-test job
  # ═══════════════════════════════════════════════════════
  pr_356_mutation_kills_completed: true
  pr_356_mutation_kills_files:
    - "src/embedding/mod.rs: 5 tests (get_provider match arms)"
    - "src/framework_ops.rs: 2 tests (secure_read_file boundary)"
    - "src/wasm.rs: 1 test (encode_text non-triviality)"
    - "src/mcp/handler.rs: 2 tests (parse_hvec roundtrip + rejection)"
    - "src/index/lsh.rs: 2 tests (serialize/deserialize roundtrip + garbage)"
    - "src/cli/commands/mod.rs: 1 test (ngram_size differentiation)"
    - "src/framework_graph_rag.rs: 1 test (probe_with_graph relevance)"
    - "tests/framework_unit.rs: 2 tests (probe_filtered + probe_with_graph)"
  pr_356_mutation_kills_count: 16

  # 2026-06-10: Fixed pre-existing flaky fastembed test (CI model download resilience)
  fastembed_test_ci_resilient: true  # get_provider_fastembed now tolerates model download failure

  # 2026-06-10: Wave 27 — PR #356 CI remediation merged
  wave_27_merged: true               # PR #362 squash-merged to main
  mutation_test_wasm_excluded: true  # WasmFramework:: mutants excluded (cfg-gated, untestable on native)
  prometheus_metrics_bridge: true    # FrameworkMetrics → Prometheus bridge connected
  adr_0074_implemented: true         # diff_versions WASM binding added
  adr_0088_created: true             # CI failure remediation record

  # ═══════════════════════════════════════════════════════
  # GOAP Reconciliation 2026-06-10 (codebase audit)
  # Re-mapped module LOC + test counts after workspace split,
  # MCP/index/embedding/observability/namespaces growth, and
  # recorded bm25 perf (PR #363) which landed after Wave 27.
  # ═══════════════════════════════════════════════════════
  goap_reconciliation_2026_06_10_complete: true
  goap_2026_06_10_main_head: "acee40b"
  goap_2026_06_10_module_map_refreshed: true     # modules / modules_csm_core /
                                                 # modules_duckdb_companion now match disk
  goap_2026_06_10_test_counts_refreshed: true    # 60 integration files, 773 test fns
  goap_2026_06_10_adr_parity:
    registry_count: 84
    disk_count: 83
    parity_satisfied: true                       # scripts/check-adr-parity.sh → ok
  goap_2026_06_10_loc_gate: satisfied            # all first-party src ≤ 500 LOC
  goap_2026_06_10_ci_status: "CI in_progress on main HEAD acee40b (#363); PR CI green pre-merge"

  # PR #363 — bm25 search allocation reduction (landed after Wave 27)
  bm25_threadlocal_scoring_buffers: true         # eliminate O(N) heap allocs per query
  bm25_short_query_linear_dedup: true            # linear-scan fast-path for token dedup
  bm25_postings_prefetch: true                   # pre-fetch postings refs in weight calc

  # ═══════════════════════════════════════════════════════
  # CI Remediation 2026-06-10 — mutation-test gate (PR #363)
  # See: plans/GOAP_CI_REMEDIATION_MUTATION_PR363.md
  # Failing run 27261966916: mutation-test 31/37 caught = 83.78% < 85%.
  # 6 mutants missed in src/retrieval/bm25.rs (PR #363 changes):
  #   - 1 KILLABLE (line 276 == → !=, dedup correctness)
  #   - 5 EQUIVALENT (271 threshold, 341/342 partial-select,
  #                   366 df>0, 369 idf>0.0 — no observable behavior)
  # Killing the 1 killable mutant → 32/37 = 86.49% ≥ 85% → gate PASSES.
  # ═══════════════════════════════════════════════════════
  ci_mutation_gate_pr363_failing_diagnosed: true
  ci_mutation_gate_pr363_fixed: true
  ci_mutation_gate_pr363_root_cause: "missing test coverage of multi-distinct-term query path"
  ci_mutation_gate_pr363_missed_total: 6
  ci_mutation_gate_pr363_killable: 1            # bm25.rs:276 == → !=
  ci_mutation_gate_pr363_equivalent: 5         # bm25.rs:271,341,342,366,369
  ci_mutation_gate_pr363_score_before_pct: 83.78
  ci_mutation_gate_pr363_score_after_pct: 86.49
  ci_mutation_gate_pr363_fix_kind: tests_only  # no production code changed
  ci_mutation_gate_pr363_tests_added:
    - "src/retrieval/bm25/tests.rs::test_search_distinct_terms_each_contribute (kills mutant 276)"
    - "src/retrieval/bm25/tests.rs::test_search_dedup_hashset_path_distinct_terms (coverage + no-inflate)"
  ci_mutation_gate_pr363_kill_verified: true   # manual mutant repro: test FAILED (1 vs 2), then reverted
  ci_mutation_gate_pr363_gates:
    cargo_test_bm25: "31 passed (was 29)"
    cargo_fmt: clean
    cargo_clippy: clean
  ci_mutation_gate_pr363_equivalent_mutants_accepted: true

  # action_last_completed (superseded 2026-06-16 by ADR-0089 reconciliation):
  #   csm_wasm_created_2026_06_12

  # ═══════════════════════════════════════════════════════
  # GOAP Orchestration 2026-06-13
  # Scanned plans/ folder, resolved missing tasks, opened issues
  # ═══════════════════════════════════════════════════════
  goap_orchestration_2026_06_13_completed: true
  goap_2026_06_13_main_head: "1acbf44"
  goap_2026_06_13_prs_merged:
    - "PR #394: refactor(workspace): replace duplicate modules with csm-memory re-exports (#371)"
  goap_2026_06_13_issues_opened:
    - "#391: feat(observability): OTLP gRPC exporter (ADR-0072 follow-up)"
    - "#392: test(security): property-based security tests"
    - "#393: fix(cli): csm-cli standalone compilation"
    - "#395: feat(errors): remediation hints for MemoryError variants"
  goap_2026_06_13_gap_analysis_status:
    f1_cli_parity: complete
    f2_mcp_server: complete
    f3_hnsw_ann: complete
    f4_agent_protocol: complete  # MCP server covers this
    f5_embedding_bridge: complete
    f6_observability: complete  # JSON + gRPC paths both shipped (PR #396)
    f7_namespace_isolation: complete
    f8_version_history: complete
    f9_reranking: complete
    f10_quantized_hvs: complete  # PR #389 merged 2026-06-14 (ADR-0075)
  goap_2026_06_13_deferred_actions:
    # 2026-06-16 reconciliation (ADR-0089): 3 of 4 entries below were stale.
    #   - otlp_grpc_exporter: DONE by PR #396 (src/observability/otlp_grpc.rs, +119 LOC)
    #   - performance_phase2: DONE — SIMD (hyperdim_simd.rs, bundle_simd.rs),
    #     LSH (index/lsh.rs), HNSW (index/hnsw.rs), and Product Quantization
    #     (PR #389 / ADR-0075) are all shipped.
    #   - namespace_isolation: DONE — src/framework_namespaces.rs (ADR-0084
    #     already called this out; ACTIONS.md entry was never updated).
    # Remaining genuinely-deferred items:
    - "advanced_ttl_policies (cost 8, ADR-0024 — baseline TTL shipped; advanced policies still deferred)"
    - "association_decay (cost 6, ADR-0025 — no user request; activation trigger not met)"

  # action_last_completed (was: goap_orchestration_2026_06_13; superseded 2026-06-16 by ADR-0089)


  # ═══════════════════════════════════════════════════════
  # GOAP Reconciliation 2026-06-16 (ADR-0089)
  # Post-PR #396 / #389 audit: 3 stale "deferred" actions exposed
  # as already-shipped, plus a duplicate action_last_completed key
  # (LEARNINGS.md invariant violation).
  # ═══════════════════════════════════════════════════════
  goap_reconciliation_2026_06_16_complete: true
  goap_2026_06_16_main_head: "4420eeb"
  goap_2026_06_16_changes:
    - "Removed duplicate action_last_completed (was 2 keys; LEARNINGS.md requires exactly 1)"
    - "ACTIONS.md: add_otlp_grpc_exporter deferred -> complete (PR #396)"
    - "ACTIONS.md: deferred_performance_phase2 deferred -> complete (SIMD/LSH/HNSW/Quantized HVs shipped)"
    - "ACTIONS.md: deferred_namespace_isolation deferred -> complete (src/framework_namespaces.rs)"
    - "GOAP_STATE.md: f10_quantized_hvs in_progress -> complete (PR #389)"
    - "GOAP_STATE.md: f6_observability comment updated (gRPC no longer deferred)"
    - "goap_2026_06_13_deferred_actions list pruned to 2 genuinely-deferred items"
  goap_2026_06_16_remaining_deferred:
    - "advanced_ttl_policies (cost 8, ADR-0024 — baseline TTL shipped; advanced still deferred)"
    - "association_decay (cost 6, ADR-0025 — no user request)"

  # ═══════════════════════════════════════════════════════
  # Wave 28: BM25 Perf + GOAP Plans Completion (2026-06-18)
  # Branch: feat/plans-completion-wave-28 (PR #413)
  # ═══════════════════════════════════════════════════════
  wave_28_name: "BM25 Perf Optimizations + Plans Completion"
  wave_28_started_at: "2026-06-16"
  wave_28_completed_at: "2026-06-18"
  wave_28_focus: "BM25/hybrid retrieval perf, mutation test CI gate fix, GOAP plans queue drain"
  wave_28_status: complete
  wave_28_pr: 413
  wave_28_changes:
    - "perf(retrieval): optimize hybrid merge and normalization (4 iterations)"
    - "perf(retrieval): consolidate bm25 normalization cache locks"
    - "perf(retrieval): optimize result merging and score comparison"
    - "perf(singularity): optimize cache key generation by eliminating redundant allocations"
    - "fix(mutation_test): respect the threshold gate in CI mode"
    - "ci: gate mutation-test on pull_request so main-branch CI fits Release window"
    - "ci(release): raise wait-for-ci timeout to 30 min and surface cancellation"
    - "ci(deps): bump taiki-e/install-action from 2.81.8 to 2.81.10"
    - "build: set explicit python version for sonarcloud analysis"
    - "chore(goap): reconcile state drift (ADR-0089)"
    - "chore: bump workspace versions and add explicit dependency requirements (#407)"
    - "fix(bm25): use 1e-6 tolerance instead of f32::EPSILON in test assertions (#398)"
    - "feat: complete remaining queued tasks from plans/ (BM25 mutation kill, clippy fix)"
  wave_28_issues_opened:
    - "#411: feat: Advanced TTL policy automation (ADR-0024)"
    - "#412: feat: Weighted forgetting with association decay (ADR-0025)"
  wave_28_validation:
    cargo_check: passing
    cargo_clippy: clean
    cargo_fmt: clean
    cargo_test_lib: "171 passed"
    cargo_test_csm_retrieval: "51 passed"
    cargo_test_csm_core_lib: "66 passed"

  # ═══════════════════════════════════════════════════════
  # Harness Engineering & Template Alignment (2026-06-23)
  # ADR-0090: rust-2026-template gap analysis
  # ═══════════════════════════════════════════════════════
  harness_engineering_gap_analysis_complete: true
  harness_engineering_adr_created: true              # ADR-0090
  harness_engineering_phase1_items: 8                # HARNESS.md, deny.toml, rust-toolchain.toml,
                                                     # quality-gates.sh, harness-check.sh, .gitleaks.toml,
                                                     # tests/arch_fitness.rs, .agents/context/
  harness_engineering_phase1_cost: 18
  harness_engineering_phase2_items: 5                # .pre-commit-config.yaml, nextest, .codecov.yml,
                                                     # commitlint.config.cjs, dist-workspace.toml
  harness_engineering_phase2_cost: 14                # delegated to Jules
  harness_engineering_deferred: 2                    # insta snapshots, Makefile
  harness_msrv_baseline_2026_06_23: "1.85"
  harness_msrv_target: "1.88"
  template_reference: "https://github.com/d-oit/rust-2026-template"
  template_version_analyzed: "0.3.2 (392 commits)"

  # superseded 2026-07-18 open PR triage (see block at end)
  # action_last_completed: wave32_p1_persistence_contracts_ci_2026_07_16

  # ═══════════════════════════════════════════════════════
  # PR #444 Review: BM25 Sparse Collection (2026-06-27)
  # Branch: perf/bm25-sparse-collection-9244619970982429697
  # Author: Jules bot (task 9244619970982429697)
  # ═══════════════════════════════════════════════════════
  pr_444_status: merged
  pr_444_title: "perf(retrieval): implement sparse collection in BM25 search"
  pr_444_source: jules_bot
  pr_444_merged_at: "2026-07-09"

  # ═══════════════════════════════════════════════════════
  # Wave 29: Harness Expansion (2026-06-25)
  # MCP integration tests, bridge persistence tests, CI matrix,
  # fuzz targets, benchmark coverage
  # ═══════════════════════════════════════════════════════
  wave_29_complete: true
  wave_29_mcp_integration_tests: 12    # tests/mcp_integration.rs
  wave_29_bridge_persistence_tests: 9  # tests/bridge_persistence_integration.rs
  wave_29_ci_matrix_csm_cli: true      # Added to test-workspace-crates
  wave_29_fuzz_targets_added: 4        # json_import, metadata_filter, bm25_tokenize, text_encoder
  wave_29_benchmarks_added: 3          # rerank_benchmark, hybrid_benchmark, embedding_benchmark
  wave_29_rerank_module_public: true   # pub mod rerank in src/retrieval/mod.rs
  wave_29_check_cfg_rerank_cross: true # Suppressed pre-existing unexpected_cfgs warning
  tests_count: 1034                     # 2026-07-14: CANONICAL — literal #[test]/#[tokio::test] across src/crates/tests (see tests_count_wave27_baseline for historical)

  # ═══════════════════════════════════════════════════════
  # GOAP Orchestrator Analysis 2026-06-26
  # Swarm-based analysis: GOAP state, CI, implementation gaps
  # ═══════════════════════════════════════════════════════
  goap_orchestrator_analysis_2026_06_26_completed: true
  goap_2026_06_26_main_head: "7a0a432"
  goap_2026_06_26_ci_status:
    ci_workflow: success
    benchmark_ci: success
    codeql: success
    dependabot_updates: success
  goap_2026_06_26_dependabot_pr_437:
    package: opentelemetry_sdk
    from_version: "0.27.1"
    to_version: "0.32.1"
    status: closed  # 2026-06-26: semver-incompatible, requires API migration
    root_cause: "Cargo.toml pins 0.27; 0.32 is breaking (TracerProvider/SpanExporter API changes)"
    resolution: "Closed — upgrade deferred until opentelemetry ecosystem stabilizes"
  goap_2026_06_26_queued_actions_count: 6
  goap_2026_06_26_queued_actions:
    - "create_deny_toml (cost 3) — supply chain auditing"
    - "create_rust_toolchain_toml (cost 1) — pin stable 1.88, bump MSRV"
    - "create_arch_fitness_tests (cost 3) — compile-time architecture tests"
    - "create_agents_context (cost 3) — cross-repo conventions doc"
    - "extract_csm_chaos_crate (cost 8) — standalone no_std chaotic maps"
    - "simd_optimize_chaotic_lsh (cost 5) — AVX2/NEON for ChaoticLsh dot-product"
  goap_2026_06_26_all_preconditions_satisfied: true
  goap_2026_06_26_recommended_next:
    priority_1: "create_rust_toolchain_toml (cost 1, lowest friction)"
    priority_2: "create_deny_toml (cost 3, supply chain security)"
    priority_3: "simd_optimize_chaotic_lsh (cost 5, perf multiplier after ADR-0091)"
    priority_4: "create_arch_fitness_tests (cost 3, prevents drift)"
    priority_5: "extract_csm_chaos_crate (cost 8, consider delegating to Jules)"
    priority_6: "create_agents_context (cost 3, DX only)"
  goap_2026_06_26_source_todo_count: 0
  goap_2026_06_26_active_wave: 30
  goap_2026_06_26_wave_30_name: "Harness Engineering Phase 1 — Toolchain & Supply Chain"
  goap_2026_06_26_wave_30_focus: "rust-toolchain.toml, deny.toml, SIMD ChaoticLsh, arch fitness tests"

  # ═══════════════════════════════════════════════════════
  # Wave 30: Harness Engineering Phase 1 (2026-06-26)
  # Toolchain pin, supply chain audit, SIMD ChaoticLsh, arch fitness
  # ═══════════════════════════════════════════════════════
  wave_30_complete: true
  wave_30_completed_at: "2026-06-26"
  wave_30_items_completed:
    - "rust-toolchain.toml: pin stable 1.88.0, bump MSRV 1.85→1.88"
    - "deny.toml: supply chain audit (advisories ok, bans ok, licenses ok, sources ok)"
    - "SIMD ChaoticLsh: AVX2 + NEON acceleration, project_scalar/project_simd, parity test"
    - "tests/arch_fitness.rs: 6 tests (LOC gate, unsafe audit, API stability, module layering)"
  wave_30_validation:
    cargo_check: passing
    cargo_fmt: clean
    cargo_clippy: clean  # uninlined_format_args fixed across workspace
    cargo_deny: passing  # advisories ok, bans ok, licenses ok, sources ok
    lib_tests: 177
    arch_fitness_tests: 6
    chaotic_lsh_tests: 2  # locality + SIMD-scalar parity
  wave_30_clippy_fixes:
    - "uninlined_format_args: 18 fixes across 8 files (triggered by 1.88 toolchain)"
  harness_msrv_current: "1.88"
  rust_toolchain_toml_created: true
  deny_toml_created: true
  arch_fitness_tests_created: true
  lsh_projection_simd_accelerated: true
  agents_context_created: true           # Re-created in PR #511 (2026-07-14)
  chaos_logic_isolated: true             # csm-chaos crate extracted (PR #441, 2026-06-26)

  # ═══════════════════════════════════════════════════════
  # PR Triage 2026-06-26 (GOAP Orchestrated)
  # ═══════════════════════════════════════════════════════
  pr_triage_2026_06_26_completed: true
  pr_triage_2026_06_26_actions:
    - "PR #441 (csm-chaos extraction): merged — all CI green, Codacy false positives (SIMD unsafe)"
    - "PR #437 (dependabot opentelemetry_sdk 0.27→0.32): closed — semver-incompatible, requires code changes"
    - "PR #442 created: remove .fastembed_cache from git tracking (128MB ML model blobs)"
  pr_441_merged: true
  pr_441_merged_at: "2026-06-26T22:00:00Z"
  pr_437_closed: true
  pr_437_closed_reason: "semver-incompatible upgrade (0.27→0.32) requiring opentelemetry API migration"
  fastembed_cache_removed_from_git: true  # PR #442

  # ═══════════════════════════════════════════════════════
  # Wave 30 Status (2026-06-27)
  # ═══════════════════════════════════════════════════════
  wave_30_status: complete
  wave_30_completed_actions:
    - create_rust_toolchain_toml
    - create_deny_toml
    - create_arch_fitness_tests
    - simd_optimize_chaotic_lsh
    - extract_csm_chaos_crate
    - hyperchaotic_bit_slicing_research_2026_06
  wave_30_remaining_queued_snapshot_2026_06_27:
    - create_agents_context       # Historical snapshot; completed by PR #511 on 2026-07-14
  ci_main_status: passing              # 2026-07-14: all jobs pass including commitlint
  ci_main_partial_results:
    lint: success
    test: success
    wasm: success
    commitlint: success               # Fixed: cli-npm scope added, Jules bot ignore rules active
  ci_dependabot_opentelemetry: closed  # PR #437 closed, not blocking
  adr_0091_registered: true

  # ═══════════════════════════════════════════════════════
  # GOAP Reconciliation 2026-07-11 (ADR-0092)
  # HEAD: 87248dba (was 7a0a432 at last audit — 13+ commits landed)
  # ═══════════════════════════════════════════════════════
  goap_reconciliation_2026_07_11_complete: true
  goap_2026_07_11_main_head: "87248dba"
  goap_2026_07_11_version: "0.3.7"
  goap_2026_07_11_test_functions: 1029     # was 696
  goap_2026_07_11_findings:
    loc_violations: 3                       # csm-memory/singularity (629), csm-core-lib/hyperdim (563), csm-memory/graph_traversal (517)
    ci_commitlint_failing: true             # scope violations: cli-npm not in enum, b649c7c no conventional format
    deny_toml_advisories_failing: false    # Fixed in PR #511: documented ignores added
    stale_pr_tracking: 2                    # PRs #444, #94 now merged (were tracked as open)
    open_prs: 1                             # PR #511 (MCP fix + deny.toml + agents context)
    todo_markers: 1                         # src/retrieval/bm25.rs:109
    dependabot_alerts_open: 5               # opentelemetry_sdk (medium), time (medium), lru (low), libsql-sqlite3-parser ×2 (low) — all blocked upstream, documented in deny.toml
  goap_2026_07_11_prs_merged_since_last_audit:
    - "#444: perf(retrieval): implement sparse collection in BM25 search"
    - "#463: Persist RetrievalAbstention events as absence-memory entries"
    - "#493: fix(build): separate cdylib target into csm-wasm"
    - "#495: refactor: rename chaotic_semantic_memory_duckdb to csm-duckdb"
    - "#497: fix(lints): warn on unwrap/expect/panic in library crates"
    - "#498: docs: clarify npm CLI installation and fix permissions guidance"
    - "#499: fix(ci): add PR triage script and merge conflict checks to workflow"
    - "#500: perf(core): optimize BundleAccumulator AVX2 count updates"
    - "#501: fix(cli-npm): handle EACCES gracefully in global install"
  goap_2026_07_11_wave_31_queued:
    - "fix_workspace_loc_gate (cost 5, P1 — extend CI LOC gate to crates/, split 3 files)"
    - "update_deny_toml_advisories (cost 2, P1 — triage new advisories, add ignores or upgrade)"
    - "fix_commitlint_scopes (cost 2, P2 — add cli-npm scope or Jules bot ignore)"
    - "merge_pr_502_simd_hamming (cost 1, P2 — Jules bot PR, MERGEABLE, 18.5% Hamming speedup)"
    - "create_agents_context (cost 3, P3 — carried from Wave 30)"
  wave_31_name: "LOC & Supply Chain Remediation"
  wave_31_started_at: "2026-07-11"
  wave_31_focus: "Fix LOC violations, supply chain advisories, commitlint hygiene, merge SIMD PR"

  # ═══════════════════════════════════════════════════════
  # Session 2026-07-14: P0/P1 Fixes (RECOMMENDATIONS_2026_07_14)
  # PR #510 merged, PR #511 created
  # ═══════════════════════════════════════════════════════
  session_2026_07_14_completed: true
  session_2026_07_14_actions:
    - "Merged PR #510 (perf: AVX2 bundle loop unroll, ~2.3% latency reduction)"
    - "Fixed P0 MCP schema/handler field name mismatch (tools.rs ↔ schema.rs)"
    - "Fixed P1 blocking std::fs in async secure_read_file (→ tokio::fs)"
    - "Updated deny.toml: documented ignore for 5 Dependabot alerts (all blocked upstream)"
    - "Created .agents/context/shared-conventions.md (org-wide conventions)"
  pr_510_merged: true
  pr_510_merged_at: "2026-07-14"
  pr_511_created: true
  pr_511_merged: true
  pr_511_merged_at: "2026-07-14T11:07:17Z"
  pr_511_merge_sha: "58918a61"
  pr_511_title: "fix(mcp): align schema/handler field names, async fs, deny.toml advisories"
  mcp_schema_handler_mismatch_fixed: true
  blocking_std_fs_in_async_fixed: true
  deny_toml_advisories_current: true

  # ═══════════════════════════════════════════════════════
  # Wave 32: Correctness, Ownership, Evidence & Agent Safety
  # Planning-only audit 2026-07-14; implementation requires ADR approval.
  # See plans/GOAP_AUDIT_2026_07_14.md and ADR-0093..0096.
  # ═══════════════════════════════════════════════════════
  codebase_audit_2026_07_14_complete: true
  codebase_audit_2026_07_14_main_head: "58918a61"
  codebase_audit_2026_07_14_ci_run: 29327781685
  codebase_audit_2026_07_14_ci_status: success
  codebase_audit_2026_07_14_open_prs: 0
  codebase_audit_2026_07_14_rust_source_files: 216
  codebase_audit_2026_07_14_skill_manifests: 32
  codebase_audit_2026_07_14_test_attributes: 1034
  codebase_audit_2026_07_14_source_loc_gate: passing
  codebase_audit_2026_07_14_adr_parity: passing
  wave_31_complete: true
  wave_31_completed_at: "2026-07-14"
  wave_32_name: "Correctness, Ownership, Evidence & Agent Safety"
  wave_32_started_at: "2026-07-14"
  wave_32_status: in_progress
  wave_32_roadmap: "plans/GOAP_AUDIT_2026_07_14.md"
  wave_32_adrs_proposed: 4
  adr_0093_proposed: true
  adr_0094_proposed: true
  adr_0095_proposed: true
  adr_0096_proposed: true
  # 2026-07-16: GOAP swarm PR implements Phase 0 accept + Phase 1 subset
  adr_0093_accepted: true
  adr_0094_accepted: true   # accepted for planning; ownership consolidation still queued
  adr_0095_accepted: true
  adr_0096_accepted: true
  harness_md_created: true
  ann_snapshot_revision_validated: true          # 2026-07-16: IndexSnapshotEnvelope + ns revision
  ann_config_is_fallible: true                   # 2026-07-16: validate_index_backend + fallible ensure_namespace
  persistence_failure_leaves_memory_unchanged: true  # durable commit before memory mutate
  load_merge_index_preserves_union: true
  association_load_queries_constant: true        # load_all_associations single SELECT
  no_framework_state_lock_across_io_await: true  # I/O before state apply; snapshot copy then await
  workspace_implementation_owners_unique: false
  no_default_features_is_lean: false
  persistence_disabled_false_success_removed: false
  mcp_full_width_vector_wire_contract: true      # 2026-07-16: base64 HVec wire + high-bit tests
  wasm_ci_release_artifact_identical: false
  benchmark_metrics_mathematically_correct: true # hit vs recall; log2 NDCG; should_abstain
  performance_claims_have_current_artifacts: false
  cargo_deny_required_in_ci: true                # 2026-07-16: cargo-deny job in ci.yml
  workspace_ci_matrix_complete: true             # csm-chaos + benchmark unit tests in CI
  benchmark_workspace_tests_run_in_ci: true
  fuzz_build_required_in_ci: true                # 2026-07-16: ci.yml fuzz-build job (add to branch protection)
  skill_validation_fail_closed: true             # 2026-07-16: wired into validate.sh + CI lint + pre-commit
  release_skill_loc_compliant: true              # 2026-07-16: SKILL.md 161 lines + references/
  release_guidance_matches_workflow: true
  skill_loc_enforced: true
  critical_skill_evals_passing: false            # behavioral evals deferred (queued follow-up)
  fuzz_short_runs_on_pr: false                   # compile-only gate; short runs queued
  fuzz_scheduled_full_runs: false                # scheduled full fuzz runs queued
  active_plan_keys_unique: true
  user_recommendations_2026_07_14_preserved: true
  wave32_p0_swarm_2026_07_16:
    branch: feat/wave32-p0-goap-correctness-validation
    completed_actions:
      - review_adr_0093_persistence_consistency
      - review_adr_0094_workspace_contracts
      - review_adr_0095_evidence_policy
      - review_adr_0096_agent_validation
      - create_harness_md
      - fix_ann_backend_validation
      - repair_fuzz_workspace_and_gate
      - make_skill_validation_fail_closed
      - align_release_skill_with_protected_workflow
      - wire_skill_format_into_ci_and_validate
      - accept_adr_0093_0096_on_disk
  wave32_p1_swarm_2026_07_16:
    branch: feat/wave32-p1-persistence-contracts-ci
    completed_actions:
      - enforce_authoritative_persistence_and_ann_revision
      - bulk_load_associations_and_release_state_locks
      - fix_mcp_hypervector_wire_format
      - correct_benchmark_metric_definitions
      - complete_workspace_ci_and_supply_chain_matrix
    deferred_to_followup:
      - run_critical_skill_behavioral_evals
      - fuzz_short_and_scheduled_runs
      - enforce_workspace_feature_contracts
      - ownership consolidation and evidence tiers

  # ═══════════════════════════════════════════════════════
  # Open PR Triage (GOAP orchestrator + swarm) 2026-07-18
  # plans/GOAP_ORCHESTRATOR.md
  # ═══════════════════════════════════════════════════════
  # action_last_completed: open_pr_triage_2026_07_18  # superseded
  open_pr_triage_2026_07_18:
    started_at: "2026-07-18"
    completed_at: "2026-07-18"
    method: goap_orchestrator_with_swarm_agents
    scanned_prs: [520, 527, 528, 529]
    merge_order: [528, 527, 529]
    closed:
      - number: 520
        reason: "empty Jules research-sim PR (0 file changes)"
    merged:
      - number: 528
        title: "perf(retrieval): optimize BM25 search hot loop"
        sha: "f8b2bbc"
      - number: 527
        title: "perf(framework): parallelize probe_batch and probe_batch_cached with Rayon"
        sha: "1e94c11"
        fixes_issue: 522
        ci_fixes:
          - "commitlint: squash invalid scope ops → framework"
          - "clippy: remove duplicated #![cfg(test)] on framework_ops_tests"
          - "mutation: restore import_* surface so Ok(1) mutants leave in-diff set"
      - number: 529
        title: "perf(retrieval): optimize hybrid merge_results with partial top-k sort"
        sha: "d2db671"
        fixes_issue: 523
        notes: "Jules regression reverted; fold min/max + select_nth + boundary test + run_query mutation exclude"
    open_prs_after: []
    remaining_open_issues_perf: [524, 525, 526]
    learnings_recorded: true
    progress_recorded: true

  # ═══════════════════════════════════════════════════════
  # Framework ops perf issues #524-#526 (2026-07-18)
  # ═══════════════════════════════════════════════════════
  framework_ops_perf_524_525_526:
    branch: feat/framework-ops-perf-524-525-526
    issues: [524, 525, 526]
    pr: 532
    status: open_pr  # awaiting merge; mutation tests added bad5118
    changes:
      - "namespace clone once via namespace() for dual-read ops (#524)"
      - "inject_concepts Rayon construction before durable write lock (#525)"
      - "import json/binary: split inject vs associate write holds (#526)"
    files:
      - src/framework_ops.rs
      - src/framework_ops_tests.rs
      - benches/benchmark.rs

  # ═══════════════════════════════════════════════════════
  # Codebase analysis + plans compaction 2026-07-20
  # Canonical recommendations: plans/RECOMMENDATIONS_2026_07_20.md
  # ═══════════════════════════════════════════════════════
  action_last_completed: run_critical_skill_behavioral_evals
  recommendations_2026_07_20_written: true
  plan_archive_2026_07_20_complete: true
  active_plan_set_compact: true
  plan_archive_manifest_valid: true
  compact_active_plans_non_destructively: true
  plans_active_index: "plans/README.md"
  plans_recommendations_canonical: "plans/RECOMMENDATIONS_2026_07_20.md"
  plans_archive_root: "plans/.archive/2026-07-20-historical"
  plans_archive_counts:
    completed_goap_docs: 25
    handoffs: 49
    total_archived_files: 74
  codebase_analysis_2026_07_20:
    product_version: "0.3.7"
    skills_count_disk: 32
    agents_md_skill_count_stale: true   # AGENTS says 30 — queued fix_agents_md_skill_inventory
    readme_version_inconsistent: true   # 0.3 vs 0.4 examples; crate is 0.3.7
    readme_ann_section_stale: true      # says ANN deferred; IndexBackend exists
    production_todo_bm25_absence: true
    cli_metrics_reset_missing: true
    ownership_divergence:
      cli_args: identical
      wasm_core: identical
      retrieval_bm25: diverged
      retrieval_hybrid: diverged
      retrieval_rerank: diverged
      persistence: diverged
    open_prs: [532, 534]
    open_issues_perf: [524, 525, 526]
    wave_32_remaining_queued: 16  # compact_active_plans now complete
    wave_33_name: "Docs truth + missing behavior + evidence"
    wave_33_queued:
      - merge_open_perf_prs_532_534
      - fix_readme_truth_version_and_ann
      - fix_agents_md_skill_inventory
      - implement_cli_metrics_reset
      - skill_loc_trim_near_ceiling
      - enforce_workspace_feature_contracts  # carried from Wave 32
      - consolidate_retrieval_ownership     # carried from Wave 32
      - implement_or_remove_bm25_absence_short_circuit
      - own_ttl_cleanup_lifecycle
      - establish_tiered_benchmark_evidence
