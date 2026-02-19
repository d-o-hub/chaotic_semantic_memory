# [ADR-0033] WASM Panic Safety: Replace Unwrap with Error Propagation

## Status
Proposed

## Context and Problem Statement
WASM bindings in `src/wasm.rs` use `.unwrap()` on `js_sys::Reflect::set()` calls throughout the metrics snapshot and probe result construction (lines 54-57, 89-92, 99-160, 237-248). While `Reflect::set` rarely fails in practice, a panic across the WASM boundary is catastrophic — it aborts the entire WASM module instance with no recovery possible.

## Decision Drivers
- Panics in WASM are unrecoverable and crash the host application
- JS Reflect operations can fail under exotic conditions (frozen objects, proxies)
- The fix is mechanical and low-risk

## Considered Options
- Option A: Replace `.unwrap()` with `.map_err(to_js_error)?` on all Reflect::set calls
- Option B: Use `serde-wasm-bindgen` for automatic serialization to JS objects
- Option C: Accept the risk (document as known limitation)

## Decision Outcome
Chosen option: "Option A — Replace `.unwrap()` with error propagation", because it's the minimal change that eliminates panic risk without adding new dependencies.

### Implementation
1. Change all `Reflect::set(...).unwrap()` calls to `Reflect::set(...).map_err(|_| JsValue::from_str("failed to set JS property"))?`
2. Ensure all affected methods return `Result<_, JsValue>`
3. For methods already returning `JsValue` (not Result), wrap in a helper that returns Result and convert at boundary

### Positive Consequences
- Zero panic risk from JS object construction
- Consistent error handling across all WASM bindings

### Negative Consequences
- Slightly more verbose code in WASM bindings (~15 lines changed)
