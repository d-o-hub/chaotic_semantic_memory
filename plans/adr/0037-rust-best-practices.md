# ADR-0037: Rust Best Practices

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

Rust best practice gaps:
- Missing #[must_use] on constructors
- Unsafe blocks without SAFETY comments
- Per-file clippy suppressions (too broad)
- CLI JSON uses format! (not serde)

## Decision

Implement **Rust best practices**.

**Deliverables:**
- #[must_use] on public constructors (HVec10240::*, Singularity::new, Framework::builder)
- SAFETY comments on SIMD blocks (alignment, bounds, aliasing)
- Per-loop clippy suppressions (not file-wide)
- CLI JSON: serde_json::json! macro

## Consequences

### Positive
- Compiler warnings for unused results
- Documented unsafe safety arguments
- Targeted clippy suppressions
- Correct JSON escaping

### Negative
- More boilerplate in code
- Requires updating existing suppressions
- #[must_use] may cause warnings

## Implementation

- Module: src/hyperdim.rs, src/singularity.rs, src/reservoir.rs, src/framework.rs
- Pattern: #[must_use] annotations, SAFETY: comments
- CLI: src/cli/commands/*.rs

## Sources

- ACTIONS.md lines 1444-1511 (Phase 23 actions)
- ADR_REGISTRY.md: Rust Best Practices details
- src/hyperdim.rs: SAFETY comments