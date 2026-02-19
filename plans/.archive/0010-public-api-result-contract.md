# [ADR-0010] Public API Result Contract

## Status
Accepted

## Context and Problem Statement
`AGENTS.md` defines a hard constraint: "All public APIs return `Result<T, Error>`."

The crate currently exposes multiple infallible public functions that return plain values (for example `HVec10240::random() -> HVec10240`, `Singularity::get() -> Option<&Concept>`, and `wasm::random_hypervector() -> Box<[u8]>`). This creates ambiguity:
- If interpreted literally, the current API violates the constraint.
- If interpreted as "all fallible public APIs return `Result`", the current API mostly complies, but the constraint text is misleading.

This ADR decides how to interpret and enforce the constraint going forward so the public surface is consistent and testable.

## Decision Drivers
- Obey the repo hard constraints in `AGENTS.md`.
- Keep the public API ergonomic for Rust callers and for WASM bindings.
- Avoid breaking changes unless there is a clear benefit and a migration plan.
- Keep error handling explicit (no silent drops of errors).

## Considered Options
- **Option A: Literal enforcement**. Change every public function to return `Result`, even if infallible (wrap in `Ok(...)`).
- **Option B: Fallible-only enforcement**. Treat the constraint as "all fallible public APIs return `Result`"; infallible APIs may return plain values.
- **Option C: Facade-only enforcement**. Enforce `Result` only for the crate-level facade (`ChaoticSemanticFramework`, `Persistence`, and externally re-exported entrypoints).

## Decision Outcome
Chosen option: **Option B: Fallible-only enforcement**, because it preserves Rust ergonomics and avoids meaningless `Ok(...)` wrappers, while still keeping failure modes explicit where they exist.

This requires updating the constraint wording in `AGENTS.md` to remove ambiguity and adding a lightweight CI/API audit to prevent regressions.

### Positive Consequences
- Public APIs stay idiomatic (pure computations remain plain-returning).
- Error paths remain explicit and testable for operations that can fail.
- Fewer breaking changes across examples/tests/benches and downstream users.

### Negative Consequences
- Requires changing the wording of a "hard constraint" to match intent.
- Requires agreement on what counts as "fallible" for builders/FFI helpers.

## Pros and Cons of the Options

### Option A: Literal enforcement
- Good, because it strictly matches the constraint text.
- Bad, because it degrades API ergonomics and adds pervasive `Ok(...)` noise.
- Bad, because it causes broad breaking changes with little functional value.

### Option B: Fallible-only enforcement
- Good, because it matches standard Rust practice and minimizes churn.
- Good, because it keeps the "Result everywhere it matters" intent.
- Bad, because it requires clarifying the constraint text and auditing for consistency.

### Option C: Facade-only enforcement
- Good, because it contains policy to a small public surface.
- Bad, because it is difficult to define mechanically and still leaves ambiguous APIs.
- Bad, because re-exports make the "facade" boundary fuzzy in practice.

## Follow-ups
- Update `AGENTS.md` constraint wording to: "All fallible public APIs return `Result<T, Error>`."
- Add a GOAP action to review and fix any public APIs that silently drop errors (for example, `ConceptBuilder::with_metadata`).
- Add a lightweight CI check or clippy policy to flag new public fallible APIs that do not return `Result`.
