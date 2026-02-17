# Swarm Coordination

## Active Swarm Groups

| Group | Phase | Focus | Status |
|-------|-------|-------|--------|
| A | 9 | Turso Latency Benchmarking | Complete |
| B | 9 | Memory Footprint Validation | Complete |
| C | 9 | WASM Size Gate | Complete |
| D | 9 | Performance Gate Integration | Complete |

## Orchestrator Run (2026-02-17)

Run ID: `goap_parallel_missing_tasks_wave5_2026_02_17`

### Parallel Wave 1 (launched)

| Group | Action | Status | Output/Handoff Artifact |
|-------|--------|--------|--------------------------|
| A | `create_fuzzing_targets` | Complete | `fuzz/` corpus + crash triage notes |
| B | `implement_simd_hypervector_ops` | Complete | benchmark deltas + invariant checklist |
| C | `add_structured_logging` | Complete | tracing field conventions + span map |
| D | `add_schema_migration_support` | Complete | migration plan + schema version map |

### Handoff Contract

1. `A -> B`: Provide malformed-input edge cases discovered by fuzzing before SIMD implementation is finalized.
2. `B -> D`: Provide data-layout compatibility notes for batch ops/cache impacts before versioning and export/import work.
3. `C -> A/B/D`: Publish tracing field names and error-context standards before merging each group’s APIs.
4. `D -> All`: Publish schema version increments and migration constraints before integration tests run.

### Phase Boundary Gates

1. After Phase 5 outputs: run full validation and update `GOAP_STATE.md`.
2. After Phase 6 outputs: rerun perf gate and update benchmark deltas.
3. After Phase 7 outputs: verify observability footprint and error context coverage.
4. Before Phase 8 merge: require migration compatibility check + restore/import validation.
5. Before Wave 5 completion: require all performance targets to pass and update `benchmarks_prove_performance`.

### Parallel Wave 2 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `expand_edge_case_coverage` | complete | W1 A |
| B | `add_connection_pooling` | complete | W1 B, W1 A handoff |
| C | `improve_error_context` | complete | W1 C |
| D | `implement_export_import` | complete | W1 D, W1 B handoff |

### Parallel Wave 3 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `enable_mutation_testing` | complete | W2 A |
| B | `add_framework_batch_operations` | complete | W2 B |
| C | `add_metrics_collection` | complete | W2 C |
| D | `add_concept_versioning` | complete | W2 D |

### Parallel Wave 4 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| B | `implement_concept_lru_cache` | complete | W3 B |
| C | `create_derive_macros` | complete | W3 C |
| D | `implement_backup_restore` | complete | W3 D |

### Parallel Wave 5 (completed)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `benchmark_turso_roundtrip` | complete | W4 complete |
| B | `validate_memory_footprint_10m` | complete | W4 complete, W5 A handoff |
| C | `validate_wasm_binary_size` | complete | W4 complete |
| D | `enforce_performance_goal_gate` | complete | W5 A, W5 B, W5 C handoffs |

### Wave 5 Handoff Contract

1. `A -> B`: Provide Turso latency profile and query mix assumptions for footprint workload alignment.
2. `B -> D`: Provide memory-accounting method and pass/fail evidence for 10M-under-12MB target.
3. `C -> D`: Provide wasm artifact path, measured size, and deterministic CI command.
4. `D -> All`: Publish final performance-goal gate decision with remediation if a target fails.

### Wave 5 Artifacts

- `plans/handoffs/W5_A_to_B_turso_latency_profile.md`
- `plans/handoffs/W5_B_to_D_memory_budget_report.md`
- `plans/handoffs/W5_C_to_D_wasm_size_report.md`
- `plans/handoffs/W5_D_to_All_performance_gate_decision.md`

## Coordination Rules

1. **Independent Operation**: Groups work on different phases without blocking
2. **ADR Gate**: Any architecture change requires ADR review before implementation
3. **Integration Points**: Phase boundaries require cross-group validation
4. **Conflict Resolution**: First-come-first-served on shared files, coordinate via GOAP_STATE
5. **Configurability Rule**: No hardcoded runtime settings or magic numbers; use named constants and env/config tunables.

## Work Distribution

### Group A: Testing & Quality
- Property-based testing (`proptest`)
- Fuzzing targets (`cargo-fuzz`)
- Edge case coverage

### Group B: Performance
- SIMD hypervector operations
- Connection pooling for Turso
- Framework batch APIs
- LRU concept cache

### Group C: Observability
- Structured logging (`tracing`)
- Metrics collection
- ~~Derive macros~~ (removed - unused)
- Error context enhancement

### Group D: Advanced Features
- Export/import (JSON + binary)
- Concept versioning
- Schema migrations
- Backup/restore operations

## Communication Protocol

1. Before starting: Read `GOAP_STATE.md` to check current status
2. During work: Update `GOAP_STATE.md` with `in_progress` flags
3. On completion: Mark actions complete, update LOC counts
4. On conflict: Document in `SWARM_ISSUES.md` (create if needed)
