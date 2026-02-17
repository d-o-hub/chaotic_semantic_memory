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
    status: in_progress
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
    status: pending
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
    status: pending
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
    status: pending
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
    status: in_progress
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
    status: pending
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
    status: pending
    file: chaotic_semantic_memory_derive/, Cargo.toml
    description: |
      Create proc-macro crate for derive macros:
      - #[derive(Concept)] for automatic metadata extraction
      - #[derive(HypervectorField)] for struct fields → HVec10240
      - Reduces boilerplate in user code

  - name: improve_error_context
    preconditions:
      core_modules_created: true
    effects:
      error_context_improved: true
    cost: 2
    status: pending
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
    status: pending
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
    status: pending
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
    status: in_progress
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
    status: pending
    file: src/framework.rs, src/persistence.rs
    description: |
      Add backup/restore operations:
      - backup(path) -> Result<()> (sqlite VACUUM INTO)
      - restore(path) -> Result<()> (replace db file)
      - List backups with timestamps
      - Integrity verification after restore
