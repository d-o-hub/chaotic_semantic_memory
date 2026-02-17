# [ADR-0022] WASM API Parity and Persistence Stub Completeness

## Status
Accepted

## Context and Problem Statement
The native API has grown significantly with export/import, backup/restore, concept_history, batch operations, clear_all, schema_version, and apply_migrations. However, WASM persistence stubs in `persistence_wasm.rs` and WASM bindings in `wasm.rs` only cover the original API surface. This creates compile errors when downstream code references missing methods and limits WASM utility.

## Decision Drivers
- Cross-platform compile safety: missing stubs cause target-specific build failures
- JS developer experience: WASM bindings should expose core functionality
- Maintenance cost: more stubs means more code to keep in sync

## Considered Options
- Minimal stubs only (current)
- Full stub parity for persistence + expanded wasm.rs bindings
- Feature-flag approach with conditional compilation

## Decision Outcome
Chosen option: "Full stub parity for persistence + expanded wasm.rs bindings", because compile parity is a hard requirement and the stub cost is minimal.

### Implementation
- Add missing persistence_wasm.rs stubs: `clear_all`, `get_concept_history`, `schema_version`, `apply_migrations`, `backup`, `restore`
- Expand wasm.rs bindings: `delete_concept`, `associate`, `get_associations`, `metrics_snapshot`
- File-based operations (backup/restore/export_file) return error on WASM
- Add `ConceptVersion` stub type for WASM parity

### Positive Consequences
- Compile parity across native and wasm32 targets
- Broader WASM utility for browser-based applications
- Consistent error messaging across platforms

### Negative Consequences
- More stub code to maintain (~30 additional lines)
- File-based operations remain unavailable on WASM (by design)
