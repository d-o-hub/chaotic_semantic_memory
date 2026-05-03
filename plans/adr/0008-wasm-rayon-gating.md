# ADR-0008: WASM Rayon Gating

## Status

Accepted (backfilled 2026-05-01)

## Context

Rayon parallel library does not compile to WASM:
- par_iter() fails in wasm32-unknown-unknown target
- WASM has no native threading (without special flags)
- Parallel code causes build failures

## Decision

Add **cfg gates on all Rayon usage** with sequential fallbacks.

**Rationale:**
- `#[cfg(not(target_arch = "wasm32"))]` for parallel code
- `#[cfg(target_arch = "wasm32")]` for sequential fallback
- Same algorithm, different execution
- WASM build succeeds with full API

## Consequences

### Positive
- WASM builds successfully
- API parity between native and WASM
- Sequential fallback is correct (just slower)
- No runtime panics in WASM

### Negative
- WASM performance lower than native
- Duplicate code paths (parallel/sequential)
- Requires testing both paths

## Implementation

- Modules: `src/hyperdim.rs`, `src/reservoir.rs`, `src/singularity.rs`
- Pattern: cfg_attr for conditional compilation
- WASM: sequential loops, native: par_iter()

## Sources

- ACTIONS.md lines 355-366 (wasm_rayon_guards action)
- Supersedes ADR-0003 (original WASM approach)
- src/hyperdim.rs: cfg gates on SIMD/Rayon