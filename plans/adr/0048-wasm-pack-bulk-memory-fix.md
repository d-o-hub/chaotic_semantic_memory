# ADR-0048: WASM-pack Bulk Memory Fix

## Status

Accepted (backfilled 2026-05-01) - 2026-02-27

## Context

WASM-pack build failures:
- wasm-opt validation errors
- Bulk memory operations not enabled
- WASM binary fails optimization pass
- Error: "invalid opcode" in wasm-opt

## Decision

Fix **WASM-pack bulk memory support**.

**Deliverables:**
- WASM_OPT_FLAGS: "--enable-bulk-memory --enable-sign-ext"
- wasm-opt requires these flags for Rust WASM bulk memory ops
- WASM build succeeds with optimization

## Consequences

### Positive
- WASM build succeeds
- wasm-opt passes validation
- Optimized WASM binary
- Bulk memory operations supported

### Negative
- wasm-opt flags required in all builds
- May need updates for wasm-opt versions
- WASM size increases slightly

## Implementation

- File: .github/workflows/npm-publish.yml
- Flags: WASM_OPT_FLAGS environment variable
- Pattern: wasm-opt --enable-bulk-memory --enable-sign-ext

## Sources

- Git: fix(wasm): resolve wasm-pack bulk memory error (2026-02-27)
- ADR_REGISTRY.md: WASM-pack Bulk Memory Fix
- .github/workflows/npm-publish.yml