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

optimization_goals:
  name: "Optimize Core Subsystems"
  targets:
    # Phase 1: Correctness (cost: 6, risk: high if unfixed)
    - permute_shift_zero_bug: false
    - reservoir_to_hvec_div_zero: false
    - associations_allow_duplicates: false
    - load_silently_overwrites: false
    - reservoir_not_reset_between_sequences: false
    - sqlite_foreign_keys_not_enforced: false
    - conceptbuilder_swallows_metadata_errors: false
    - libsql_deprecated_apis_used: false

    # Phase 2: Performance (cost: 22, impact: high)
    - reservoir_dense_matrix_infeasible: false   # ADR-0004
    - singularity_search_sequential: false       # ADR-0007
    - reservoir_step_per_alloc: false
    - bundle_per_chunk_alloc: false
    - persistence_no_batching: false             # ADR-0006
    - persistence_connection_unsafe: false       # ADR-0005

    # Phase 3: Capabilities (cost: 12, impact: medium)
    - wasm_rayon_not_gated: false                # ADR-0008 (supersedes ADR-0003)
    - no_concept_deletion_in_framework: false
    - no_memory_limits: false
    - prelude_module_missing: false
    - no_integration_tests: false

enhancement_goals:
  name: "Architecture Enhancements (future triggers)"
  targets:
    - approximate_search_indexing: false  # trigger: >200k concepts + latency miss
    - deterministic_hashed_projections: false  # trigger: w_in memory pressure

improvement_goals:
  name: "Continuous Improvement Program"
  targets:
    # Phase 5: Testing & Quality (cost: 8)
    - property_based_tests_added: false
    - fuzzing_targets_created: false
    - edge_case_coverage_complete: false
    - mutation_testing_enabled: false

    # Phase 6: Performance (cost: 12)
    - simd_hypervector_ops: false
    - connection_pooling_turso: false
    - framework_batch_operations: false
    - concept_cache_implemented: false

    # Phase 7: Observability & DX (cost: 10)
    - structured_logging_added: false
    - metrics_collection_enabled: false
    - derive_macros_created: false
    - error_context_improved: false

    # Phase 8: Advanced Features (cost: 15)
    - export_import_functionality: false
    - concept_versioning_enabled: false
    - schema_migration_support: false
    - backup_restore_operations: false
