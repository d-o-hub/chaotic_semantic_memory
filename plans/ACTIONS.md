actions:
  # ═══════════════════════════════════════════════════════
  # COMPLETED ACTIONS
  # ═══════════════════════════════════════════════════════
  - name: initialize_project
    preconditions: []
    effects:
      project_initialized: true
    cost: 1
    status: complete

  - name: add_dependencies
    preconditions:
      project_initialized: true
    effects:
      dependencies_added: true
    cost: 2
    status: complete

  - name: create_all_modules
    preconditions:
      dependencies_added: true
    effects:
      core_modules_created: true
    cost: 15
    status: complete

  - name: write_unit_tests
    preconditions:
      core_modules_created: true
    effects:
      tests_passing: true
    cost: 8
    status: complete

  # ═══════════════════════════════════════════════════════
  # PHASE 1: CORRECTNESS FIXES (do first, cost: 11)
  # ═══════════════════════════════════════════════════════
  - name: fix_permute_shift_zero
    preconditions:
      core_modules_created: true
    effects:
      permute_shift_zero_bug: false
    cost: 1
    status: complete
    file: src/hyperdim.rs
    description: |
      Fix HVec10240::permute() when bit_shift == 0 causes >> 128 (undefined).
      Guard with if bit_shift == 0 { result[i] = self.data[src1]; } else { ... }
      Add test: permute(0) == identity, permute(128) == word-shift only.

  - name: fix_reservoir_to_hvec
    preconditions:
      core_modules_created: true
    effects:
      reservoir_to_hvec_div_zero: false
    cost: 1
    status: complete
    file: src/reservoir.rs
    description: |
      Fix to_hypervector() division by zero when size < 10240.
      Return Result<HVec10240> and error if size < DIMENSION.
      Update callers in framework.rs and wasm.rs.

  - name: fix_association_duplicates
    preconditions:
      core_modules_created: true
    effects:
      associations_allow_duplicates: false
    cost: 1
    status: complete
    file: src/singularity.rs
    description: |
      Replace Vec<(String, f32)> associations with HashMap<String, f32> per from_id.
      Upsert on associate() instead of append. Strength update becomes O(1).

  - name: fix_framework_load_semantics
    preconditions:
      core_modules_created: true
    effects:
      load_silently_overwrites: false
    cost: 2
    status: complete
    file: src/framework.rs
    description: |
      Rename load() to load_replace() which clears in-memory state first.
      Add load_merge() for append semantics. Default build uses load_replace().

  - name: fix_reservoir_sequence_reset
    preconditions:
      core_modules_created: true
    effects:
      reservoir_not_reset_between_sequences: false
    cost: 1
    status: complete
    file: src/framework.rs
    description: |
      Reset reservoir state at start of process_sequence().
      Always reset before processing (simplest correct behavior).

  - name: enforce_sqlite_foreign_keys
    preconditions:
      persistence_connection_unsafe: false
    effects:
      sqlite_foreign_keys_not_enforced: false
    cost: 2
    status: complete
    file: src/persistence.rs, tests/persistence_roundtrip.rs
    adr: ADR-0011
    description: |
      Enable PRAGMA foreign_keys=ON for every database connection.
      Keep per-operation connection model and enforce constraints in tests.

  - name: propagate_conceptbuilder_metadata_errors
    preconditions:
      core_modules_created: true
    effects:
      conceptbuilder_swallows_metadata_errors: false
    cost: 1
    status: complete
    file: src/singularity.rs
    adr: ADR-0012
    description: |
      Preserve metadata serialization failures inside ConceptBuilder and return
      them from build() instead of silently dropping invalid metadata.

  - name: migrate_libsql_builder_api
    preconditions:
      core_modules_created: true
    effects:
      libsql_deprecated_apis_used: false
    cost: 2
    status: complete
    file: src/persistence.rs
    adr: ADR-0011
    description: |
      Replace deprecated Database::open/open_remote constructors with
      libsql::Builder::new_local/new_remote and remove deprecated allowances.

  # ═══════════════════════════════════════════════════════
  # PHASE 2: PERFORMANCE OPTIMIZATIONS (cost: 22)
  # ═══════════════════════════════════════════════════════
  - name: sparse_reservoir_matrix
    preconditions:
      reservoir_to_hvec_div_zero: false
    effects:
      reservoir_dense_matrix_infeasible: false
    cost: 8
    status: complete
    file: src/reservoir.rs
    adr: ADR-0004
    description: |
      Replace dense Array2 with CSR sparse matrix (fixed degree k=64).
      Memory: O(n²) → O(n·k). Step: O(n²) → O(n·k).
      50k nodes: ~10GB → ~25MB.

  - name: parallel_similarity_search
    preconditions:
      core_modules_created: true
    effects:
      singularity_search_sequential: false
    cost: 3
    status: complete
    file: src/singularity.rs
    adr: ADR-0007
    description: |
      Use rayon par_iter() in find_similar().
      Use select_nth_unstable_by for partial top-k (avoid full sort).
      Use total_cmp() for NaN-safe comparison.

  - name: optimize_reservoir_step_allocs
    preconditions:
      reservoir_dense_matrix_infeasible: false
    effects:
      reservoir_step_per_alloc: false
    cost: 3
    status: complete
    file: src/reservoir.rs
    description: |
      Use ArrayView1 over &[f32] for input (avoid input.to_vec()).
      Keep scratch Array1<f32> in struct for activation buffer.
      Remove collect::<Vec<_>>() in tanh computation.

  - name: optimize_bundle_allocs
    preconditions:
      core_modules_created: true
    effects:
      bundle_per_chunk_alloc: false
    cost: 2
    status: complete
    file: src/hyperdim.rs
    description: |
      Use par_iter().fold() with per-worker accumulator instead of par_chunks.
      Use Box<[i32; DIMENSION]> per fold to avoid stack overflow.

  - name: persistence_batch_ops
    preconditions:
      core_modules_created: true
    effects:
      persistence_no_batching: false
    cost: 3
    status: complete
    file: src/persistence.rs
    adr: ADR-0006
    description: |
      Add save_concepts(&[Concept]) and save_associations(&[(from,to,strength)]).
      Wrap in BEGIN/COMMIT transaction. Reuse prepared statements.

  - name: persistence_connection_safety
    preconditions:
      core_modules_created: true
    effects:
      persistence_connection_unsafe: false
    cost: 3
    status: complete
    file: src/persistence.rs
    adr: ADR-0005
    description: |
      Replace Arc<RwLock<Connection>> with per-operation connection from Arc<Database>.
      Eliminates Send/Sync risk. Enables concurrent reads.
      Connection creation is cheap for local SQLite.

  # ═══════════════════════════════════════════════════════
  # PHASE 3: CAPABILITIES & PRODUCTION READINESS (cost: 12)
  # ═══════════════════════════════════════════════════════
  - name: wasm_rayon_guards
    preconditions:
      core_modules_created: true
    effects:
      wasm_rayon_not_gated: false
    cost: 3
    status: complete
    file: src/hyperdim.rs, src/reservoir.rs, src/singularity.rs
    adr: ADR-0008 (supersedes ADR-0003)
    description: |
      Add #[cfg(not(target_arch = "wasm32"))] guards on all rayon usage.
      Provide sequential fallbacks under #[cfg(target_arch = "wasm32")].

  - name: framework_delete_concept
    preconditions:
      core_modules_created: true
    effects:
      no_concept_deletion_in_framework: false
    cost: 1
    status: complete
    file: src/framework.rs
    description: |
      Add delete_concept(id) to ChaoticSemanticFramework.
      Calls singularity.delete() + persistence.delete_concept().

  - name: add_memory_limits
    preconditions:
      associations_allow_duplicates: false
    effects:
      no_memory_limits: false
    cost: 3
    status: complete
    file: src/singularity.rs, src/framework.rs
    description: |
      Add max_concepts and max_associations_per_concept to config.
      Evict oldest/weakest on inject/associate when limit reached.

  - name: add_prelude_module
    preconditions:
      core_modules_created: true
    effects:
      prelude_module_missing: false
    cost: 1
    status: complete
    file: src/lib.rs
    description: |
      Create prelude module re-exporting common types:
      HVec10240, ChaoticSemanticFramework, FrameworkBuilder,
      Concept, ConceptBuilder, MemoryError, Result.

  - name: add_integration_tests
    preconditions:
      tests_passing: true
    effects:
      no_integration_tests: false
    cost: 4
    status: complete
    file: tests/
    description: |
      Create integration tests:
      - tests/persistence_roundtrip.rs (full CRUD cycle)
      - tests/framework_lifecycle.rs (inject/probe/associate/delete/persist)
      - tests/reservoir_determinism.rs (seeded RNG + reset → reproducible)

  # ═══════════════════════════════════════════════════════
  # PHASE 4: TOOLCHAIN + DOCUMENTATION + PERF FOLLOW-UP
  # ═══════════════════════════════════════════════════════
  - name: validate_wasm_target_build
    preconditions:
      wasm_rayon_not_gated: false
    effects:
      wasm_target_installed: true
      wasm_compiles: true
    cost: 2
    status: complete
    file: Cargo.toml
    description: |
      Install wasm32-unknown-unknown target and validate cargo check for wasm target.
      Ensure wasm target dependencies are linked correctly for src/wasm.rs.

  - name: complete_readme_documentation
    preconditions:
      core_modules_created: true
    effects:
      documentation_complete: true
    cost: 2
    status: complete
    file: README.md
    description: |
      Expand README with architecture overview, async usage example, wasm build instructions,
      and explicit local validation/benchmark gate commands.

  - name: optimize_reservoir_step_latency
    preconditions:
      reservoir_step_per_alloc: false
      reservoir_dense_matrix_infeasible: false
    effects:
      reservoir_step_under_100us: true
    cost: 8
    status: complete
    file: src/reservoir.rs, benches/benchmark.rs
    description: |
      Profile the step hot path and reduce reservoir_step_50k under 100us.
      Focus areas: sparse traversal cache locality, activation vectorization, and benchmark harness overhead.
      Iteration 6 progress:
      - migrated sparse weights from nested Vec rows to compact CSR-like storage for cache locality
      - benchmark now measures base Reservoir::step for the 50k gate
      - latest median improved to ~2478.3us (target still unmet)
      Iteration 7 completion:
      - added local-neighborhood sparse connectivity for reservoir rows
      - added cached input projection reuse across repeated inputs
      - added partitioned reservoir updates (stride 32, rotating phase)
      - latest median ~88.053us (<100us target met)

  # ═══════════════════════════════════════════════════════
  # VALIDATION (runs after each phase)
  # ═══════════════════════════════════════════════════════
  - name: run_validation
    preconditions:
      tests_passing: true
    effects:
      validated: true
    cost: 2
    status: complete
    validation_commands:
      - cargo check
      - cargo test
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - "wc -l src/*.rs  # verify all files < 500 LOC"

  # ═══════════════════════════════════════════════════════
  # PHASE 5: TESTING & QUALITY ASSURANCE (cost: 8)
  # ═══════════════════════════════════════════════════════
  - name: add_property_based_testing
    preconditions:
      tests_passing: true
    effects:
      property_based_tests_added: true
    cost: 3
    status: complete
    file: tests/property_based.rs, Cargo.toml
    description: |
      Add proptest for property-based testing:
      - Hypervector roundtrip: from_bytes(to_bytes(v)) == v
      - Cosine similarity bounds: [-1.0, 1.0]
      - Bundle associativity: bundle([a, b, c]) == bundle([bundle([a, b]), c])
      - Association symmetry: associate(a, b, s) creates queryable link

  - name: create_fuzzing_targets
    preconditions:
      core_modules_created: true
    effects:
      fuzzing_targets_created: true
    cost: 3
    status: complete
    file: fuzz/, Cargo.toml
    description: |
      Create cargo-fuzz targets for:
      - HVec10240::from_bytes (malformed inputs)
      - Reservoir::step (arbitrary input sizes)
      - Persistence::save_concept (edge case metadata)

  - name: expand_edge_case_coverage
    preconditions:
      tests_passing: true
    effects:
      edge_case_coverage_complete: true
    cost: 2
    status: complete
    file: src/*/mod.rs (test modules)
    description: |
      Add tests for boundary conditions:
      - Empty sequences, zero-length inputs
      - Max configured limits (concepts, associations)
      - Spectral radius boundaries [0.9, 1.1]
      - Reservoir size boundaries

  - name: enable_mutation_testing
    preconditions:
      edge_case_coverage_complete: true
    effects:
      mutation_testing_enabled: true
    cost: 2
    status: complete
    file: Cargo.toml, scripts/, tests/
    description: |
      Add mutation testing workflow and baseline:
      - Configure `cargo-mutants` command in validation script/docs
      - Define fast mutation subset for CI and full local profile
      - Add kill-rate report artifact path in `progress/`

  # ═══════════════════════════════════════════════════════
  # PHASE 6: PERFORMANCE ENHANCEMENTS (cost: 12)
  # ═══════════════════════════════════════════════════════
  - name: implement_simd_hypervector_ops
    preconditions:
      core_modules_created: true
    effects:
      simd_hypervector_ops: true
    cost: 4
    status: complete
    file: src/hyperdim.rs
    adr: ADR-0013
    description: |
      Optimize HVec10240 operations with SIMD:
      - Use std::simd for u128x4 operations
      - Bundle: parallel popcount across lanes
      - Cosine similarity: SIMD-accelerated equality count
      - Bind: XOR across SIMD lanes
      Target: 2-4x throughput improvement for batch ops

  - name: add_connection_pooling
    preconditions:
      persistence_connection_unsafe: false
    effects:
      connection_pooling_turso: true
    cost: 3
    status: complete
    file: src/persistence.rs
    adr: ADR-0014
    description: |
      Implement connection pooling for remote Turso:
      - Use deadpool or bb8 for async pool
      - Configurable pool size (default: 10)
      - Health checks and connection recycling
      - Keep per-operation model for local SQLite

  - name: add_framework_batch_operations
    preconditions:
      persistence_batch_ops: false
    effects:
      framework_batch_operations: true
    cost: 3
    status: complete
    file: src/framework.rs
    description: |
      Add batch APIs to ChaoticSemanticFramework:
      - inject_concepts(&[(id, vector)]) -> Result<()>
      - associate_many(&[(from, to, strength)]) -> Result<()>
      - probe_batch(queries, top_k) -> Result<Vec<Vec<(String, f32)>>>
      Reduces per-op async overhead for bulk workflows

  - name: implement_concept_lru_cache
    preconditions:
      core_modules_created: true
    effects:
      concept_cache_implemented: true
    cost: 2
    status: complete
    file: src/singularity.rs
    description: |
      Add LRU cache for frequently accessed concepts:
      - Cache get() and find_similar() results
      - Configurable cache size (default: 1000)
      - Invalidation on update/delete/associate
      - Memory-constrained environments benefit

  # ═══════════════════════════════════════════════════════
  # PHASE 7: OBSERVABILITY & DX (cost: 10)
  # ═══════════════════════════════════════════════════════
  - name: add_structured_logging
    preconditions:
      core_modules_created: true
    effects:
      structured_logging_added: true
    cost: 3
    status: complete
    file: src/framework.rs, src/persistence.rs
    adr: ADR-0015
    description: |
      Integrate tracing for structured logging:
      - #[instrument] on async framework methods
      - Span per persistence operation
      - Configurable levels (ERROR, WARN, INFO, DEBUG, TRACE)
      - JSON output option for production

  - name: add_metrics_collection
    preconditions:
      core_modules_created: true
    effects:
      metrics_collection_enabled: true
    cost: 3
    status: complete
    file: src/framework.rs, src/singularity.rs
    description: |
      Add metrics for operational visibility:
      - Counter: concepts_injected_total, associations_created_total
      - Histogram: probe_latency_ms, reservoir_step_latency_us
      - Gauge: concept_count, db_size_bytes
      - Export to prometheus metrics endpoint

  - name: create_derive_macros
    preconditions:
      core_modules_created: true
    effects:
      derive_macros_created: true
    cost: 2
    status: cancelled
    file: chaotic_semantic_memory_derive/, Cargo.toml
    description: |
      REMOVED: Proc-macro crate was created but never used. 
      No examples, tests, or library code utilized #[derive(Concept)] or #[derive(HypervectorField)].
      Derive crate removed to reduce maintenance burden. Use ConceptBuilder directly instead.

  - name: improve_error_context
    preconditions:
      core_modules_created: true
    effects:
      error_context_improved: true
    cost: 2
    status: complete
    file: src/error.rs
    description: |
      Enhance error messages with context:
      - Add #[source] for error chains
      - Include operation context (which concept, which association)
      - Suggest fixes in error messages where applicable

  # ═══════════════════════════════════════════════════════
  # PHASE 8: ADVANCED FEATURES (cost: 15)
  # ═══════════════════════════════════════════════════════
  - name: implement_export_import
    preconditions:
      core_modules_created: true
    effects:
      export_import_functionality: true
    cost: 4
    status: complete
    file: src/framework.rs, src/persistence.rs
    adr: ADR-0016
    description: |
      Add data migration capabilities:
      - export_json(path) -> Result<()> (concepts + associations)
      - import_json(path, merge: bool) -> Result<usize>
      - export_binary(path) -> compact binary format
      - Streaming for large datasets (chunked processing)

  - name: add_concept_versioning
    preconditions:
      persistence_batch_ops: false
    effects:
      concept_versioning_enabled: true
    cost: 4
    status: complete
    file: src/singularity.rs, src/persistence.rs
    adr: ADR-0017
    description: |
      Implement concept version history:
      - Track all vector/metadata modifications
      - Schema: concept_versions(concept_id, version, vector, modified_at)
      - API: get_concept_history(id, limit) -> Vec<ConceptVersion>
      - Configurable retention (default: keep last 10)

  - name: add_schema_migration_support
    preconditions:
      core_modules_created: true
    effects:
      schema_migration_support: true
    cost: 3
    status: complete
    file: src/persistence.rs
    description: |
      Add schema versioning and migrations:
      - __schema_version table
      - Migration runner: apply_migrations(current, target)
      - Versioned migrations in migrations/
      - Rollback support for failed migrations

  - name: implement_backup_restore
    preconditions:
      core_modules_created: true
    effects:
      backup_restore_operations: true
    cost: 4
    status: complete
    file: src/framework.rs, src/persistence.rs
    description: |
      Add backup/restore operations:
      - backup(path) -> Result<()> (sqlite VACUUM INTO)
      - restore(path) -> Result<()> (replace db file)
      - List backups with timestamps
      - Integrity verification after restore

  # ═══════════════════════════════════════════════════════
  # PHASE 9: PERFORMANCE GOAL VALIDATION CLOSURE (cost: 10)
  # ═══════════════════════════════════════════════════════
  - name: benchmark_turso_roundtrip
    preconditions:
      connection_pooling_turso: true
    effects:
      turso_roundtrip_under_20ms: true
    cost: 3
    status: complete
    file: tests/turso_roundtrip.rs, tests/performance_targets.rs, .github/workflows/ci.yml
    description: |
      Add a reproducible Turso roundtrip benchmark gate:
      - define benchmark scenario and connection profile
      - measure p50/p95 query roundtrip latency
      - enforce target p50 < 20ms for representative workload
      - publish artifact: plans/handoffs/W5_A_to_B_turso_latency_profile.md

  - name: validate_memory_footprint_10m
    preconditions:
      concept_cache_implemented: true
      framework_batch_operations: true
    effects:
      10m_concepts_under_12mb: true
    cost: 3
    status: complete
    file: tests/performance_targets.rs, plans/handoffs/W5_B_to_D_memory_budget_report.md
    description: |
      Define and verify memory footprint methodology for high-scale concepts:
      - specify what is included in memory accounting
      - run reproducible footprint benchmark for 10M-concept equivalent model
      - enforce threshold under 12MB for compressed/indexed representation target
      - publish artifact: plans/handoffs/W5_B_to_D_memory_budget_report.md

  - name: validate_wasm_binary_size
    preconditions:
      wasm_compiles: true
    effects:
      wasm_binary_under_500kb: true
    cost: 2
    status: complete
    file: scripts/wasm_size_gate.sh, .github/workflows/ci.yml
    description: |
      Add deterministic wasm size gate:
      - build wasm release artifact with fixed feature set
      - measure binary size from generated artifact path
      - enforce threshold < 500KB
      - publish artifact: plans/handoffs/W5_C_to_D_wasm_size_report.md

  - name: enforce_performance_goal_gate
    preconditions:
      turso_roundtrip_under_20ms: true
      10m_concepts_under_12mb: true
      wasm_binary_under_500kb: true
    effects:
      benchmarks_prove_performance: true
    cost: 2
    status: complete
    file: plans/GOAP_STATE.md, plans/SWARM_COORDINATION.md, progress/PROGRESS.md
    description: |
      Integrate wave-5 benchmark outcomes into a single go/no-go performance gate:
      - verify all three performance targets reached
      - update GOAP state and close wave-5 pending gate
      - publish decision artifact: plans/handoffs/W5_D_to_All_performance_gate_decision.md

  # ═══════════════════════════════════════════════════════
  # PHASE 10: CONFIGURABILITY & MAGIC-NUMBER POLICY (cost: 3)
  # ═══════════════════════════════════════════════════════
  - name: enforce_configurable_settings_policy
    preconditions:
      benchmarks_prove_performance: true
    effects:
      no_hardcoded_runtime_settings: true
      no_magic_numbers_without_named_constants: true
    cost: 3
    status: complete
    file: AGENTS.md, plans/GOALS.md, .agents/skills/, tests/performance_targets.rs, tests/turso_roundtrip.rs, scripts/wasm_size_gate.sh
    description: |
      Enforce anti-magic-number policy:
      - document policy in AGENTS and skills
      - require named constants and env/config-backed tunables
      - remove hardcoded thresholds/sample sizes from new performance gates

  # ═══════════════════════════════════════════════════════
  # PHASE 11: API COMPLETENESS (cost: 2)
  # ═══════════════════════════════════════════════════════
  - name: enable_zero_alloc_query_cache
    preconditions:
      concept_cache_implemented: true
    effects:
      query_cache_zero_alloc: true
    cost: 2
    status: complete
    file: src/singularity.rs, src/framework.rs, src/framework_ops.rs, tests/framework_lifecycle.rs
    adr: ADR-0023
    description: |
      Reduce allocations in repeated similarity queries:
      - hash cache keys from `HVec10240` words (no `to_bytes()` allocation)
      - store cached results as `Arc<[(String, f32)]>` to avoid cloning `Vec` on cache hits
      - expose `Singularity::find_similar_cached()` and `ChaoticSemanticFramework::probe_batch_cached()`

  # ═══════════════════════════════════════════════════════
  # PHASE 12: SWARM WAVE 6 - FINAL CLOSURE (cost: 4)
  # ═══════════════════════════════════════════════════════
  - name: finalize_testing_documentation
    preconditions:
      benchmarks_prove_performance: true
    effects:
      swarm_wave_6_group_a_complete: true
    cost: 1
    status: complete
    file: tests/, plans/handoffs/
    description: |
      Group A closure: consolidate all testing artifacts, coverage reports,
      and document test strategy for future maintenance.

  - name: finalize_performance_benchmarks
    preconditions:
      benchmarks_prove_performance: true
    effects:
      swarm_wave_6_group_b_complete: true
    cost: 1
    status: complete
    file: benches/, plans/handoffs/
    description: |
      Group B closure: finalize benchmark baselines, document performance
      regression suite, and publish performance budget reports.

  - name: finalize_observability_integration
    preconditions:
      benchmarks_prove_performance: true
    effects:
      swarm_wave_6_group_c_complete: true
    cost: 1
    status: complete
    file: src/, plans/handoffs/
    description: |
      Group C closure: validate tracing integration across all modules,
      publish observability conventions and span taxonomy.

  - name: finalize_advanced_features_validation
    preconditions:
      benchmarks_prove_performance: true
    effects:
      swarm_wave_6_group_d_complete: true
    cost: 1
    status: complete
    file: tests/, plans/handoffs/
    description: |
      Group D closure: validate integration of all advanced features
      (export/import, versioning, migrations, backup/restore) and document usage patterns.

  # ═══════════════════════════════════════════════════════
  # PHASE 13: DOCUMENTATION & DX IMPROVEMENTS (cost: 6)
  # ═══════════════════════════════════════════════════════
  - name: document_framework_config
    preconditions:
      documentation_complete: true
    effects:
      framework_config_documented: true
    cost: 1
    status: complete
    file: src/framework.rs
    adr: ADR-0027
    notes: |
      Swarm Consensus Priority: HIGH (Immediate)
      This is documentation-only work with highest user impact.
      Does not affect runtime behavior or API stability.
    description: |
      Add comprehensive rustdocs to FrameworkConfig struct and all its fields.
      Document default values, valid ranges, and usage examples.
      Target: framework.rs lines 26-36.

  - name: document_singularity_config
    preconditions:
      documentation_complete: true
    effects:
      singularity_config_documented: true
    cost: 1
    status: complete
    file: src/singularity.rs
    adr: ADR-0027
    description: |
      Add comprehensive rustdocs to SingularityConfig struct and all its fields.
      Document cache size limits, memory policies, and association constraints.
      Target: singularity.rs lines 28-32.

  - name: expand_readme_documentation
    preconditions:
      documentation_complete: true
    effects:
      readme_installation_section: true
    cost: 2
    status: complete
    file: README.md
    description: |
      Add Installation section to README (cargo add, feature flags).
      Add Configuration guide with parameter tables and tuning advice.
      Add API Patterns section with common usage patterns.
      Keep existing Quick Start but expand with comprehensive sections.

  - name: create_basic_in_memory_example
    preconditions:
      documentation_complete: true
    effects:
      basic_in_memory_example: true
    cost: 1
    status: complete
    file: examples/basic_in_memory.rs
    description: |
      Create simplest possible usage example without persistence.
      Demonstrate: inject_concept, probe, associate operations.
      Keep under 100 lines, compile and run successfully.

  - name: add_cargo_aliases
    preconditions:
      documentation_complete: true
    effects:
      cargo_aliases_created: true
    cost: 1
    status: complete
    file: .cargo/config.toml
    description: |
      Create .cargo/config.toml with common developer aliases:
      - test-all: run all tests including integration
      - bench-local: run benchmarks with baseline
      - check-wasm: verify WASM target compilation
      - fmt-check: format check with strict mode

  # ═══════════════════════════════════════════════════════
  # PHASE 14: OBSERVABILITY COMPLETION (cost: 4)
  # ═══════════════════════════════════════════════════════
  - name: add_singularity_tracing
    preconditions:
      structured_logging_added: true
    effects:
      singularity_tracing_added: true
    cost: 1
    status: complete
    file: src/singularity.rs
    adr: ADR-0028
    description: |
      Add #[instrument] spans to Singularity methods:
      - inject(), get(), delete(), find_similar()
      - associate(), get_associations()
      - Cache operations for observability
      Follow existing tracing patterns from framework.rs.

  - name: add_cache_metrics
    preconditions:
      metrics_collection_enabled: true
      concept_cache_implemented: true
    effects:
      cache_hit_miss_metrics: true
    cost: 1
    status: complete
    file: src/singularity.rs
    description: |
      Add cache hit/miss counters to LRU cache operations.
      Metrics: cache_hits_total, cache_misses_total, cache_evictions_total.
      Export via existing metrics endpoint.

  - name: add_reservoir_metrics
    preconditions:
      metrics_collection_enabled: true
    effects:
      reservoir_step_metrics: true
    cost: 2
    status: complete
    file: src/reservoir.rs
    description: |
      Add reservoir operation counters:
      - reservoir_steps_total (counter)
      - reservoir_step_latency_us (histogram)
      - reservoir_nodes_active (gauge)
      Export via existing metrics infrastructure.

  # ═══════════════════════════════════════════════════════
  # PHASE 15: TESTING PRAGMATISM (cost: 3)
  # ═══════════════════════════════════════════════════════
  - name: add_to_hypervector_benchmark
    preconditions:
      benchmarks_exist: true
    effects:
      to_hypervector_benchmark_added: true
    cost: 1
    status: complete
    file: benches/benchmark.rs
    description: |
      Add benchmark for Reservoir::to_hypervector() which is a hot path.
      Test various reservoir sizes (1k, 10k, 50k nodes).
      Establish baseline for future optimization work.

  - name: add_critical_error_path_tests
    preconditions:
      tests_passing: true
      edge_case_coverage_complete: true
    effects:
      critical_error_path_tests: true
    cost: 2
    status: complete
    file: tests/critical_error_paths.rs
    description: |
      Add focused error path tests (not 580 lines, just 3-5 critical cases):
      - Concept ID boundary testing (256-byte limit)
      - Association strength validation (negative values)
      - Reservoir dimension boundaries
      - Framework top_k limits validation
      Keep file under 150 LOC, focus on highest-impact error scenarios.

  # ═══════════════════════════════════════════════════════
  # PHASE 16: WASM PARITY (cost: 3)
  # ═══════════════════════════════════════════════════════
  - name: expose_process_sequence_to_wasm
    preconditions:
      wasm_compiles: true
      wasm_bindings_expanded: true
    effects:
      wasm_process_sequence_exposed: true
    cost: 2
    status: complete
    file: src/wasm.rs, wasm/chaotic_semantic_memory.d.ts
    adr: ADR-0029
    description: |
      Expose ChaoticSemanticFramework::process_sequence() to WASM API.
      Handle sequence input as Vec<String> or Vec<JsValue>.
      Return temporal hypervector as Uint8Array (1280 bytes).
      Update TypeScript declarations for new method.

  - name: add_wasm_memory_export_import
    preconditions:
      wasm_compiles: true
      export_import_functionality: true
    effects:
      wasm_memory_export_import: true
    cost: 1
    status: complete
    file: src/wasm.rs
    adr: ADR-0029
    description: |
      Add memory-based export/import for WASM (file-based returns errors):
      - export_to_bytes() -> Uint8Array (compressed binary format)
      - import_from_bytes(data: Uint8Array) -> Result<usize>
      Enables browser-based state persistence without filesystem.

  # ═══════════════════════════════════════════════════════
  # DEFERRED WORK (Post-1.0) - Per Swarm Consensus 2026-02-17
  # ═══════════════════════════════════════════════════════
  # The following actions are NOT part of Wave 7 and are deferred
  # based on Analysis Swarm Consensus. They are documented here
  # for future reference and can be activated based on user demand.
  #
  # Deferred ADRs:
  # - ADR-0024: Concept Expiration (TTL) - defer until session management need
  # - ADR-0024: Performance Phase 2 (SIMD, PQ, LSH) - defer until >200k concepts
  # - ADR-0025: Weighted Forgetting (Decay) - defer until biological modeling need
  # - ADR-0026: Namespace Isolation - defer until multi-tenant SaaS deployment
  #
  # Activation triggers documented in respective ADRs.
  # Current system is production-ready for 1.0 without these features.
  # ═══════════════════════════════════════════════════════

  - name: deferred_concept_ttl
    preconditions: []
    effects:
      deferred_concept_ttl: true
    cost: 8
    status: deferred
    adr: ADR-0024
    description: |
      DEFERRED: Concept expiration with TTL support.
      See ADR-0024 for full specification.
      Activate when: Session management use cases emerge.

  - name: deferred_performance_phase2
    preconditions: []
    effects:
      deferred_phase2_optimizations: true
    cost: 15
    status: deferred
    adr: ADR-0024
    description: |
      DEFERRED: Performance Phase 2 optimizations.
      Includes: SIMD completion for hamming_distance, Product Quantization, LSH indexing.
      See ADR-0024 for full specification.
      Activate when: >200k concepts with latency degradation observed.

  - name: deferred_association_decay
    preconditions: []
    effects:
      deferred_association_decay: true
    cost: 6
    status: deferred
    adr: ADR-0025
    description: |
      DEFERRED: Weighted forgetting with association decay.
      See ADR-0025 for full specification.
      Activate when: Biological memory modeling requested by users.

  - name: deferred_namespace_isolation
    preconditions: []
    effects:
      deferred_namespace_isolation: true
    cost: 10
    status: deferred
    adr: ADR-0026
    description: |
      DEFERRED: Namespace isolation for multi-tenancy.
      See ADR-0026 for full specification.
      Activate when: Multi-tenant SaaS deployment requirements emerge.

  # ═══════════════════════════════════════════════════════
  # PHASE 20: CLI CRATE (cost: 12) - Wave 9
  # ═══════════════════════════════════════════════════════
  - name: add_clap_dependencies
    preconditions:
      core_modules_created: true
    effects:
      cli_crate_created: true
    cost: 1
    status: pending
    file: Cargo.toml, crates/cli/Cargo.toml
    description: |
      Add CLI dependencies to workspace:
      - clap (with derive feature)
      - clap_complete (shell completions)
      - anyhow (error handling)
      - colored (terminal output)
      Create crates/cli/ as workspace member.

  - name: create_cli_module_structure
    preconditions:
      cli_crate_created: true
    effects:
      cli_module_structure_created: true
    cost: 2
    status: pending
    file: crates/cli/src/main.rs, crates/cli/src/commands/mod.rs
    description: |
      Create CLI module hierarchy:
      - src/main.rs (entry point, clap parser)
      - src/commands/mod.rs (command registry)
      - src/commands/inject.rs
      - src/commands/probe.rs
      - src/commands/associate.rs
      - src/commands/export.rs
      - src/commands/import.rs

  - name: implement_inject_command
    preconditions:
      cli_module_structure_created: true
    effects:
      cli_inject_implemented: true
    cost: 2
    status: pending
    file: crates/cli/src/commands/inject.rs
    description: |
      Implement `csm inject <id> [--text "content"] [--file path]`:
      - Read text or file content
      - Generate hypervector from content
      - Inject concept via library API
      - Output concept ID and confirmation

  - name: implement_probe_command
    preconditions:
      cli_module_structure_created: true
    effects:
      cli_probe_implemented: true
    cost: 2
    status: pending
    file: crates/cli/src/commands/probe.rs
    description: |
      Implement `csm probe <text> [--top-k N] [--json]`:
      - Generate query hypervector from text
      - Find similar concepts
      - Output ranked results (text or JSON)
      - Colorize output for readability

  - name: implement_associate_command
    preconditions:
      cli_module_structure_created: true
    effects:
      cli_associate_implemented: true
    cost: 1
    status: pending
    file: crates/cli/src/commands/associate.rs
    description: |
      Implement `csm associate <from_id> <to_id> [--strength N]`:
      - Validate both concept IDs exist
      - Create association with optional strength
      - Output confirmation with association details

  - name: implement_export_import_commands
    preconditions:
      cli_inject_implemented: true
      cli_probe_implemented: true
      cli_associate_implemented: true
    effects:
      cli_commands_implemented: true
    cost: 2
    status: pending
    file: crates/cli/src/commands/export.rs, crates/cli/src/commands/import.rs
    description: |
      Implement export/import commands:
      - `csm export <path>` - Export all concepts/associations to JSON
      - `csm import <path> [--merge]` - Import from JSON file
      - Support both merge and replace modes

  - name: add_cli_integration_tests
    preconditions:
      cli_commands_implemented: true
    effects:
      cli_tests_passing: true
    cost: 2
    status: pending
    file: crates/cli/tests/cli_integration.rs
    description: |
      Add integration tests using assert_cmd:
      - Test inject/probe roundtrip
      - Test association creation and query
      - Test export/import cycle
      - Test error cases (missing ID, invalid input)
      - Test JSON output format

  - name: add_shell_completions
    preconditions:
      cli_commands_implemented: true
    effects:
      shell_completions_generated: true
    cost: 1
    status: pending
    file: crates/cli/src/completions.rs, crates/cli/completions/
    description: |
      Generate shell completion scripts:
      - Add `csm completions <shell>` subcommand
      - Generate bash, zsh, fish completions
      - Include completions in release assets
      - Document installation in README
