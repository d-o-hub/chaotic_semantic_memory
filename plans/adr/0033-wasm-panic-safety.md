# ADR-0033: WASM Panic Safety

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

WASM boundary panics:
- Reflect::set(...).unwrap() causes panics
- metrics_snapshot() unwraps internally
- Unrecoverable panics across WASM boundary

## Decision

Implement **WASM panic safety**.

**Deliverables:**
- Replace Reflect::set().unwrap() with error propagation
- metrics_snapshot() returns Result<JsValue, JsValue>
- All WASM errors return Err(JsValue) instead of panic

## Consequences

### Positive
- No unrecoverable panics in WASM
- Errors catchable in JavaScript
- Safe WASM boundary
- Better debugging in browser

### Negative
- WASM API more complex (Result handling)
- JavaScript must handle errors
- Some operations become fallible

## Implementation

- Module: src/wasm.rs
- Pattern: error propagation instead of unwrap
- Return: Result<JsValue, JsValue> for all methods

## Sources

- ACTIONS.md lines 1290-1301 (fix_wasm_panic_safety action)
- ADR_REGISTRY.md: WASM Panic Safety details
- src/wasm.rs: error propagation