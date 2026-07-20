primary_goal:
  name: "Production-Ready Memory System"
  targets:
    - project_initialized: true
    - all_modules_created: true
    - tests_passing: true
    - benchmarks_prove_performance: true
    - wasm_compiles: true
    - binary_built: true
    - sample_app_working: true
    - documentation_complete: true

performance_goals:
  - reservoir_step_under_100us: true
  - turso_roundtrip_under_20ms: true
  - 10m_concepts_under_12mb: true
  - wasm_binary_under_500kb: true

engineering_goals:
  - no_hardcoded_runtime_settings: true
  - no_magic_numbers_without_named_constants: true

optimization_goals:
  name: "Optimize Core Subsystems"
  targets:
    # Phase 1: Correctness - ALL COMPLETED ✓
    # Note: These were completed in earlier waves, GOAP_STATE tracks true status
    - permute_shift_zero_bug: true      # Fixed: hyperdim.rs permute(0) guard
    - reservoir_to_hvec_div_zero: true  # Fixed: proper size validation
    - associations_allow_duplicates: true  # Fixed: HashMap storage
    - load_silently_overwrites: true    # Fixed: load_replace/load_merge semantics
    - reservoir_not_reset_between_sequences: true  # Fixed: reset() at sequence start
    - sqlite_foreign_keys_not_enforced: true  # Fixed: PRAGMA foreign_keys=ON
    - conceptbuilder_swallows_metadata_errors: true  # Fixed: error propagation
    - libsql_deprecated_apis_used: true  # Fixed: Builder API migration

    # Phase 2: Performance - ALL COMPLETED ✓
    - reservoir_dense_matrix_infeasible: true   # ADR-0004 - Sparse CSR implemented
    - singularity_search_sequential: true       # ADR-0007 - Rayon parallelization
    - reservoir_step_per_alloc: true            # Fixed: scratch buffer reuse
    - bundle_per_chunk_alloc: true              # Fixed: fold() accumulator pattern
    - persistence_no_batching: true             # ADR-0006 - Batch operations
    - persistence_connection_unsafe: true       # ADR-0005 - Per-operation connections

    # Phase 3: Capabilities - ALL COMPLETED ✓
    - wasm_rayon_not_gated: true                # ADR-0008 - cfg gating
    - no_concept_deletion_in_framework: true    # Fixed: delete_concept() added
    - no_memory_limits: true                    # Fixed: max_concepts/max_associations
    - prelude_module_missing: true              # Fixed: prelude module added
    - no_integration_tests: true                # Fixed: 7 integration tests

enhancement_goals:
  name: "Architecture Enhancements (Post-1.0 Deferred)"
  note: "Deferred per Swarm Consensus 2026-02-17. Reconsider based on production feedback."
  targets:
    # ADR-0056 / ADR-0024 Phase 2B/C: Trigger when >200k concepts with latency degradation
    # after exact-scan materialization costs are removed from the probe path
    - approximate_search_indexing: deferred  
    # ADR-0024 Phase 2: Trigger when w_in memory pressure observed
    - deterministic_hashed_projections: deferred
    # ADR-0024: Product Quantization for 10M concept scale
    - product_quantization: deferred
    # ADR-0056 / ADR-0024: Optional LSH index for sub-linear search
    - lsh_indexing: deferred
    # ADR-0025: Biological memory decay modeling
    - association_decay: deferred
    # ADR-0024 baseline shipped; advanced policy automation remains deferred
    - concept_ttl: true
    # ADR-0026: Multi-tenancy support
    - namespace_isolation: deferred

improvement_goals:
  name: "Continuous Improvement Program"
  targets:
    # Phase 5: Testing & Quality (cost: 8)
    - property_based_tests_added: true
    - fuzzing_targets_created: true
    - edge_case_coverage_complete: true
    - mutation_testing_enabled: true

    # Phase 6: Performance (cost: 12)
    - simd_hypervector_ops: true
    - connection_pooling_turso: true
    - framework_batch_operations: true
    - concept_cache_implemented: true

    # Phase 7: Observability & DX (cost: 10)
    - structured_logging_added: true
    - metrics_collection_enabled: true
    - derive_macros_created: true  # Cancelled: proc-macro crate removed (unused), use ConceptBuilder directly
    - error_context_improved: true

    # Phase 8: Advanced Features (cost: 15)
    - export_import_functionality: true
    - concept_versioning_enabled: true
    - schema_migration_support: true
    - backup_restore_operations: true

    # Phase 11: API Completeness (cost: 2)
    - query_cache_zero_alloc: true

    # Phase 48: Performance Follow-up (cost: 7)
    - probe_scan_materialization_removed: true
    - local_sqlite_wal_enabled: true

release_and_validation_goals:
  name: "CI Truthfulness And Evaluation Integrity"
  targets:
    - ci_executes_real_criterion_benches: true
    - benchmark_workspace_tests_run_in_ci: true
    - wasm_js_smoke_test_enforced: true
    - wasm_docs_match_generated_api: true
    - benchmark_ci_enforces_quality_thresholds: true
    - benchmark_storage_metric_truthful: true
    - benchmark_report_contract_complete: true
    - pages_fallback_renders_html: true

# Wave 33 targets (from plans/RECOMMENDATIONS_2026_07_20.md)
wave_33_goals:
  name: "Docs Truth, Ownership, Missing Behavior, Evidence"
  note: "Queued after Wave 32 P0/P1 subset; see ACTIONS.md wave-33"
  targets:
    - workspace_implementation_owners_unique: false  # still dual-write
    - no_default_features_is_lean: false
    - bm25_absence_todo_resolved: false
    - ttl_cleanup_task_owned: false
    - readme_version_consistent: false
    - readme_ann_section_matches_code: false
    - agents_skill_count_matches_disk: false
    - cli_metrics_reset_implemented: false
    - performance_claims_have_current_artifacts: false
    - active_plan_set_compact: true   # achieved 2026-07-20
