# ADR-0091: Bridge Persistence Integration Tests

## Status

Proposed

## Context and Problem Statement

The bridge persistence module (`src/bridge_persistence.rs`, 317 LOC) provides
libSQL-backed storage for canonical concepts (save, load, delete). It has zero
integration tests. The only coverage is indirect through framework lifecycle
tests, which do not exercise the bridge persistence paths in isolation.

Functions lacking dedicated test coverage:
- `save_canonical_concept` — INSERT/REPLACE into csm_canonical_concepts
- `load_canonical_concept` — SELECT by ID with label/related deserialization
- `delete_canonical_concept` — DELETE by ID
- `load_all_canonical_concepts` — bulk load for graph reconstruction
- Error paths (missing concept, DB constraint violations)

## Decision

Create `tests/bridge_persistence_integration.rs` with tests covering:

1. **Round-trip**: save → load → verify all fields (id, labels, related_ids, version)
2. **Update**: save → modify → save again → verify updated fields
3. **Delete**: save → delete → load returns None/NotFound
4. **Bulk load**: save N concepts → load_all → verify count and contents
5. **Label deserialization**: concepts with multiple labels serialize/deserialize correctly
6. **Related IDs**: concepts with related_ids array persist and reconstruct
7. **Error handling**: load non-existent ID returns appropriate error

Estimated cost: 3

## Consequences

- Catches serialization bugs in the concept↔SQL mapping without requiring
  the full framework + bridge retrieval stack.
- Enables confident refactoring of the persistence layer.
- Aligns with the project's 93% test:source coverage target.

## References

- `src/bridge_persistence.rs` — Module under test
- `src/semantic_bridge.rs` — CanonicalConcept type definition
- ADR-0061 — Semantic Bridge Layer design
