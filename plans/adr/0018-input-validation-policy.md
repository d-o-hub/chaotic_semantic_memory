# ADR-0018: Input Validation Policy

## Status

Accepted (backfilled 2026-05-01)

## Context

Unvalidated inputs cause runtime failures:
- Concept IDs: any string accepted
- Metadata: unbounded size
- Association strength: negative values possible

## Decision

Implement **input validation with clear error messages**.

**Rationale:**
- Concept ID: max 256 bytes, no invalid characters
- Metadata: max 64KB per concept
- Association strength: [0.0, 1.0] range
- Early validation prevents runtime errors

## Consequences

### Positive
- Clear error messages for invalid input
- Prevents runtime panics
- Bounds checking protects resources
- User-friendly validation feedback

### Negative
- May reject previously accepted inputs
- Validation overhead on every operation
- Requires caller to handle validation errors

## Implementation

- Module: `src/singularity.rs`, `src/framework.rs`
- Validation: MemoryError::InvalidInput
- Bounds: documented in error messages
- Suggestion: fix hints in errors where applicable

## Sources

- ACTIONS.md lines 694-702 (improve_error_context action)
- src/error.rs: InvalidInput variants
- tests: boundary condition testing