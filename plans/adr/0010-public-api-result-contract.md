# ADR-0010: Public API Result Contract

## Status

Accepted (backfilled 2026-05-01)

## Context

API error handling consistency across modules:
- Mixed error types: anyhow, thiserror, custom
- Problem: Inconsistent error propagation
- Problem: Users cannot distinguish error types

## Decision

Use **custom MemoryError enum** with thiserror for all public APIs.

**Rationale:**
- Consistent `Result<T, MemoryError>` across library
- thiserror provides #[source] chains
- Clear error taxonomy: NotFound, InvalidInput, Database, etc.
- Users can match on specific error types

## Consequences

### Positive
- Consistent error handling across API
- Clear error categories for users
- Source chain for debugging
- #[serde] compatible error serialization

### Negative
- Requires error conversion from dependencies
- More boilerplate than anyhow
- Internal functions can use anyhow

## Implementation

- Module: `src/error.rs`
- Type: `MemoryError` enum with #[derive(thiserror::Error)]
- Result alias: `pub type Result<T> = std::result::Result<T, MemoryError>`
- Categories: NotFound, InvalidInput, Database, Capacity, Reservoir

## Sources

- ACTIONS.md lines 690-702 (improve_error_context action)
- src/error.rs: MemoryError definition
- GOAP_STATE.md: result_contract_clarified: true