# ADR-0012: ConceptBuilder Metadata Error Propagation

## Status

Accepted (backfilled 2026-05-01)

## Context

ConceptBuilder metadata handling:
- Original: Invalid metadata silently dropped
- Problem: Serialization failures not surfaced
- Problem: Users unaware of metadata issues

## Decision

**Preserve and propagate metadata errors** from build().

**Rationale:**
- Collect serialization errors during with_metadata()
- Return all errors from build() instead of dropping
- Users can see what failed and why
- Valid metadata preserved, invalid reported

## Consequences

### Positive
- No silent metadata loss
- Clear error messages for invalid metadata
- Users can fix serialization issues
- Consistent error handling pattern

### Negative
- Build may fail on previously accepted metadata
- Requires error handling in caller
- May break existing code expecting silent drops

## Implementation

- Module: `src/singularity.rs`, `src/concept_builder.rs`
- Pattern: Collect errors in Vec, return on build()
- Error type: MemoryError::InvalidMetadata
- Builder: Result<Concept> instead of Concept

## Sources

- ACTIONS.md lines 243-254 (propagate_conceptbuilder_metadata_errors action)
- src/concept_builder.rs: error collection
- src/singularity.rs: ConceptBuilder integration