# Swarm Coordination

## Active Swarm Groups

| Group | Phase | Focus | Status |
|-------|-------|-------|--------|
| A | 5 | Testing & Quality | Ready |
| B | 6 | Performance | Ready |
| C | 7 | Observability & DX | Ready |
| D | 8 | Advanced Features | Ready |

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
