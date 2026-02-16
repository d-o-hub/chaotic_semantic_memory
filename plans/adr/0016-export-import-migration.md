# [ADR-0016] Data Export/Import for Migration

## Status
Proposed

## Context and Problem Statement
Users need to:
- Backup data to external storage
- Migrate between database instances
- Export for analysis in other tools
- Import bulk data from other systems

## Decision Drivers
- Data portability is a core requirement for production systems
- Must handle large datasets (streaming/chunking)
- Must be format-agnostic where possible
- Must preserve referential integrity

## Considered Options
1. **SQL dump** - Native SQLite approach, but Turso-specific
2. **JSON export/import** - Human-readable, widely supported
3. **Binary format** - Compact, fast, but opaque
4. **Arrow/Parquet** - Analytics-friendly, but heavy dependency

## Decision Outcome
Chosen option: **Multiple formats**: JSON (human-readable), Binary (compact), streaming support

### Implementation Strategy
- `export_json(path)` - Full dump with associations
- `import_json(path, merge: bool)` - Load with conflict resolution
- `export_binary(path)` - Custom compact format
- Streaming for datasets > 10k concepts (chunked processing)
- Transaction wrapping for consistency

### Format Specification (JSON)
```json
{
  "version": "0.1.0",
  "exported_at": 1700000000,
  "concepts": [...],
  "associations": [...]
}
```

### Positive Consequences
- Data portability between deployments
- Human-readable JSON for debugging
- Compact binary for backups
- Streaming handles large datasets

### Negative Consequences
- Export can be slow for large datasets
- Binary format requires version compatibility
- JSON size is large compared to binary

## Links
- [Serde JSON](https://docs.rs/serde_json/)
