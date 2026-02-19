# [ADR-0018] Input Validation Policy for Public APIs

## Status
Accepted

## Context and Problem Statement
Public APIs accept arbitrary concept IDs (empty, extremely long), association strengths (NaN, Inf, negative), unbounded top_k values, and unlimited metadata sizes. No validation exists at the API boundary, allowing silent corruption and potential DoS via oversized inputs.

## Decision Drivers
- Security: DoS via large inputs, oversized metadata
- Data integrity: NaN poisons sorting and caching
- API ergonomics: clear error messages for invalid input
- Hard constraint: all fallible public APIs must return Result<T, Error>

## Considered Options
- No validation (current behavior)
- Strict validation at framework boundary only
- Validation at both framework and singularity layers

## Decision Outcome
Chosen option: "Strict validation at framework boundary only", because it provides a single enforcement point without duplicating checks in internal modules.

### Validation Rules
- Concept IDs: non-empty, max 256 bytes, valid UTF-8
- Association strength: must be finite (`f32::is_finite()`)
- top_k: capped at configurable maximum (default 10,000)
- Metadata: optional configurable max bytes per concept (default: unbounded for backward compatibility)

### Positive Consequences
- Prevents silent data corruption from NaN/Inf strengths
- Protects against DoS from oversized inputs
- Clear, actionable error messages at the API boundary

### Negative Consequences
- Breaking change for callers currently passing empty IDs
- Minor validation overhead on every API call
