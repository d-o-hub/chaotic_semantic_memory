# ADR-0015: Structured Logging

## Status

Accepted (backfilled 2026-05-01)

## Context

Production debugging requires structured logs:
- Original: println! and basic log macros
- Problem: No structured metadata
- Problem: Difficult to trace async operations

## Decision

Integrate **tracing for structured logging**.

**Rationale:**
- #[instrument] spans on async methods
- Per-operation span with metadata
- Configurable levels (ERROR, WARN, INFO, DEBUG, TRACE)
- JSON output option for production

## Consequences

### Positive
- Structured logs with context
- Span hierarchy for async tracing
- Filterable by level
- JSON format for log aggregation

### Negative
- Additional dependency (tracing, tracing-subscriber)
- WASM requires cfg gating
- Overhead from span creation

## Implementation

- Module: `src/framework.rs`, `src/persistence.rs`
- Spans: #[instrument] on public methods
- Levels: ERROR for failures, INFO for operations, DEBUG for details
- WASM: cfg_attr gating

## Sources

- ACTIONS.md lines 646-660 (add_structured_logging action)
- src/framework.rs: #[instrument] attributes
- src/reservoir.rs: tracing spans