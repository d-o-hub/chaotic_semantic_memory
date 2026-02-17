# Swarm Coordination

## Active Swarm Groups

| Group | Phase | Focus | Status |
|-------|-------|-------|--------|
| A | 5 | Testing & Quality | Ready (Wave 2 queued) |
| B | 6 | Performance | In Progress |
| C | 7 | Observability & DX | In Progress |
| D | 8 | Advanced Features | In Progress |

## Orchestrator Run (2026-02-17)

Run ID: `goap_parallel_missing_tasks_2026_02_17`

### Parallel Wave 1 (launched)

| Group | Action | Status | Output/Handoff Artifact |
|-------|--------|--------|--------------------------|
| A | `create_fuzzing_targets` | Complete | `fuzz/` corpus + crash triage notes |
| B | `implement_simd_hypervector_ops` | In Progress | benchmark deltas + invariant checklist |
| C | `add_structured_logging` | In Progress | tracing field conventions + span map |
| D | `add_schema_migration_support` | In Progress | migration plan + schema version map |

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

### Parallel Wave 2 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `expand_edge_case_coverage` | when `create_fuzzing_targets` complete | W1 A |
| B | `add_connection_pooling` | when `implement_simd_hypervector_ops` complete | W1 B, W1 A handoff |
| C | `improve_error_context` | when `add_structured_logging` complete | W1 C |
| D | `implement_export_import` | when `add_schema_migration_support` complete | W1 D, W1 B handoff |

### Parallel Wave 3 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `enable_mutation_testing` | when `expand_edge_case_coverage` complete | W2 A |
| B | `add_framework_batch_operations` | when `add_connection_pooling` complete | W2 B |
| C | `add_metrics_collection` | when `improve_error_context` complete | W2 C |
| D | `add_concept_versioning` | when `implement_export_import` complete | W2 D |

### Parallel Wave 4 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| B | `implement_concept_lru_cache` | when `add_framework_batch_operations` complete | W3 B |
| C | `create_derive_macros` | when `add_metrics_collection` complete | W3 C |
| D | `implement_backup_restore` | when `add_concept_versioning` complete | W3 D |

## Coordination Rules

1. **Independent Operation**: Groups work on different phases without blocking
2. **ADR Gate**: Any architecture change requires ADR review before implementation
3. **Integration Points**: Phase boundaries require cross-group validation
4. **Conflict Resolution**: First-come-first-served on shared files, coordinate via GOAP_STATE

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
- Derive macros
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
