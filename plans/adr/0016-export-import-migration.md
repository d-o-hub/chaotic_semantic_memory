# ADR-0016: Export/Import Migration

## Status

Accepted (backfilled 2026-05-01)

## Context

Data migration between instances requires export/import:
- Original: No migration capability
- Problem: Cannot move data between environments
- Problem: No backup/restore mechanism

## Decision

Add **export/import for JSON and binary formats**.

**Rationale:**
- export_json(path) -> concepts + associations
- import_json(path, merge: bool) -> Result<usize>
- export_binary(path) -> compact binary (bincode)
- Streaming for large datasets (chunked processing)

## Consequences

### Positive
- Data migration between instances
- Backup/restore capability
- JSON for debugging, binary for efficiency
- Merge mode for incremental import

### Negative
- Large datasets require streaming
- Binary format version compatibility
- Import validation overhead

## Implementation

- Module: `src/framework.rs`, `src/persistence.rs`
- Formats: JSON (serde_json), Binary (bincode)
- Streaming: chunked processing for >10k concepts
- Merge: append semantics vs replace

## Sources

- ACTIONS.md lines 707-721 (implement_export_import action)
- ADR-0058: serialization fixes (Base64 for HVec)
- src/framework.rs: export/import methods