# ADR-0022: WASM API Parity (Original)

## Status

Accepted (backfilled 2026-05-01)

## Context

WASM bindings need API parity with native:
- Original: Limited WASM API (inject, probe only)
- Problem: Missing methods in WASM
- Problem: Different behavior from native

## Decision

Ensure **WASM API parity** with native framework.

**Rationale:**
- wasm-bindgen for JavaScript bindings
- All public methods exposed
- Uint8Array for hypervector data
- Error propagation (not panics)

## Consequences

### Positive
- WASM users have full API
- Consistent behavior with native
- JavaScript-friendly types
- No panics across WASM boundary

### Negative
- WASM-specific implementation for some methods
- Performance lower than native
- Async patterns differ in WASM

## Implementation

- Module: `src/wasm.rs`
- Bindings: wasm-bindgen attributes
- Types: JsValue conversions
- Safety: error propagation (see ADR-0033)

## Sources

- ACTIONS.md: WASM API development
- src/wasm.rs: wasm-bindgen exports
- MEMORY.md: WASM size gate (~870KB)