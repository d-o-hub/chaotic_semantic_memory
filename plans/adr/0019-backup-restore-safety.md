# ADR-0019: Backup/Restore Safety

## Status

Accepted (backfilled 2026-05-01)

## Context

Backup/restore operations risk data corruption:
- Original: No backup mechanism
- Problem: Cannot recover from failures
- Problem: Restore may corrupt existing data

## Decision

Add **safe backup/restore with integrity verification**.

**Rationale:**
- backup(path) -> Result<()> (SQLite VACUUM INTO)
- restore(path) -> Result<()> (replace db file)
- List backups with timestamps
- Integrity verification after restore

## Consequences

### Positive
- Safe backup mechanism
- Integrity verification prevents corruption
- Timestamps for backup management
- SQLite VACUUM INTO for compact backup

### Negative
- Backup requires disk space
- Restore replaces entire database
- Integrity check adds overhead

## Implementation

- Module: `src/framework.rs`, `src/persistence.rs`
- Backup: VACUUM INTO for SQLite
- Restore: file replacement + verification
- Verification: checksum comparison

## Sources

- ACTIONS.md lines 754-767 (implement_backup_restore action)
- src/persistence.rs: backup/restore methods
- tests: backup/restore roundtrip