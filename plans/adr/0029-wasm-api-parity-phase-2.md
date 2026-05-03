# ADR-0029: WASM API Parity (Phase 2)

## Status

Accepted (backfilled 2026-05-01) - Wave 7 Complete

## Context

WASM API missing features:
- process_sequence() not exposed
- Export/import require filesystem (errors in browser)
- Memory-based persistence needed

## Decision

Complete **WASM API parity** with memory-based operations.

**Deliverables:**
- process_sequence() exposed to WASM (Vec<String> input, Uint8Array output)
- export_to_bytes() -> Uint8Array (compressed binary)
- import_from_bytes(data: Uint8Array) -> Result<usize>
- TypeScript declarations updated

## Consequences

### Positive
- Full WASM API parity
- Browser-based persistence (no filesystem)
- TypeScript type safety
- WASM users have full functionality

### Negative
- WASM-specific implementation
- Binary format complexity
- Uint8Array size limits

## Implementation

- Module: src/wasm.rs, wasm/chaotic_semantic_memory.d.ts
- Pattern: memory-based I/O (Uint8Array)
- Size: WASM binary ~870KB (under 1MB gate)

## Sources

- ACTIONS.md lines 1082-1112 (Phase 16 actions)
- ADR_REGISTRY.md: Wave 7 Active ADRs
- wasm/test.js: smoke tests