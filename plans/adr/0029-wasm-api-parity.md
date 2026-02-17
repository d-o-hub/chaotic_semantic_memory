# ADR-0029: WASM API Parity - Exposing Missing Methods

## Status
- **Proposed**: 2026-02-17
- **Accepted**: 2026-02-17

**Priority**: High (Immediate - Wave 7)

**Rationale**: Analysis Swarm Consensus identified `process_sequence()` as core differentiator missing from WASM API. Memory-based export/import enables browser persistence (file-based doesn't work in WASM). Critical for WASM feature parity.

## Context

The WASM API surface is incomplete compared to the native Rust API. Critical missing functionality:
1. `process_sequence()` - Temporal processing via reservoir (key feature)
2. Memory-based export/import - File-based APIs don't work in browser environments

This limits WASM use cases to basic CRUD operations, missing the temporal reasoning capabilities that distinguish this crate.

## Decision

### 1. Expose process_sequence() to WASM

**Interface:**
```rust
#[wasm_bindgen]
impl ChaoticSemanticFramework {
    /// Process a sequence of tokens through the reservoir.
    /// Returns the temporal hypervector as a Uint8Array.
    ///
    /// # Arguments
    /// * `tokens` - Array of token strings to process
    /// * `steps_per_token` - Reservoir steps between tokens (default: 10)
    #[wasm_bindgen(js_name = processSequence)]
    pub fn process_sequence_wasm(
        &mut self,
        tokens: Vec<String>,
        steps_per_token: Option<usize>,
    ) -> Result<js_sys::Uint8Array, JsValue> {
        // ... implementation
    }
}
```

**TypeScript declarations:**
```typescript
export class ChaoticSemanticFramework {
    processSequence(
        tokens: string[],
        stepsPerToken?: number
    ): Uint8Array;
}
```

### 2. Memory-Based Export/Import

**Rationale:** File-based export/import (`export_json`, `import_json`) return errors in WASM since browsers don't have filesystem access.

**New methods:**
```rust
/// Export all data to a compressed binary format in memory.
/// Returns Uint8Array suitable for IndexedDB or localStorage.
#[wasm_bindgen(js_name = exportToBytes)]
pub fn export_to_bytes(&self) -> Result<js_sys::Uint8Array, JsValue>;

/// Import data from bytes (previously exported via exportToBytes).
/// Returns number of concepts imported.
#[wasm_bindgen(js_name = importFromBytes)]
pub fn import_from_bytes(
    &mut self,
    data: js_sys::Uint8Array,
    merge: bool,
) -> Result<usize, JsValue>;
```

**Format:** Reuse existing binary export format (compressed, versioned).

### 3. Implementation Constraints

- **No Rayon in WASM**: Sequential fallbacks already exist
- **Error handling**: Convert Rust errors to JsValue with context
- **Memory management**: Transfer ownership of Uint8Array to JS
- **Size limit**: Keep WASM binary < 500KB (current: ~350KB)

## Consequences

### Positive
- Full temporal processing available in browsers
- State persistence via IndexedDB/localStorage
- Feature parity between native and WASM APIs
- Enables browser-based AI memory applications

### Negative
- +30-50 lines to wasm.rs (still under 500 LOC)
- Slightly larger WASM binary (~5KB increase)
- Additional testing needed for browser environments

### Alternative Considered
**Skip process_sequence in WASM**: Rejected - temporal processing is a core differentiator, omitting it makes WASM bindings much less valuable.

## Implementation Plan

1. Add `process_sequence_wasm()` wrapper (Day 1)
2. Add `export_to_bytes()` and `import_from_bytes()` (Day 1)
3. Update TypeScript declarations (Day 1)
4. Test in browser environment (Day 2)
5. Add example: `examples/wasm_browser.html` (Day 2)

## Compliance

- **500 LOC limit**: wasm.rs currently 165 LOC, additions bring to ~210 LOC
- **WASM binary < 500KB**: Current ~350KB, increase ~5KB
- **No hardcoded settings**: Uses existing config
- **libsql only**: No new database dependencies

## References

- Analysis artifact: `plans/handoffs/analysis_group_d_features.md`
- Current WASM API: `src/wasm.rs`
- Binary format: `src/persistence.rs` (export_binary)
- ADR-0008: WASM Rayon gating (ensures compatibility)
