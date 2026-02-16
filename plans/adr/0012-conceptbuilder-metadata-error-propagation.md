# [ADR-0012] ConceptBuilder Metadata Error Propagation

## Status
Accepted

## Context and Problem Statement
`ConceptBuilder::with_metadata` serialized values into `serde_json::Value` and silently ignored serialization errors. This hid invalid input and violated explicit error-handling expectations.

## Decision Drivers
- Avoid silent data loss.
- Keep the fluent builder API.
- Keep public error handling consistent with crate `Result` patterns.

## Considered Options
1. Keep current behavior (swallow errors).
2. Add `try_with_metadata(...) -> Result<Self>`.
3. Capture metadata serialization errors and return them from `build()`.

## Decision Outcome
Chosen option: **Capture serialization errors during `with_metadata` and return them from `build()`**.

### Positive Consequences
- Metadata failures are explicit and testable.
- Existing fluent call sites remain unchanged.
- Error type stays in existing `MemoryError::Serialization`.

### Negative Consequences
- Some existing call paths that previously "succeeded with dropped metadata" now fail at build time.
- Failure point is deferred to `build()` rather than the `with_metadata` call site.

## Pros and Cons of the Options

### Swallow errors
- Good, because the API looks simple.
- Bad, because data corruption is silent.

### `try_with_metadata`
- Good, because errors surface immediately.
- Bad, because this introduces a parallel builder API and migration complexity.

### Defer to `build()`
- Good, because it preserves fluent chaining and still surfaces errors.
- Good, because implementation impact is small.
- Bad, because the exact failing metadata field is not returned as structured context yet.
