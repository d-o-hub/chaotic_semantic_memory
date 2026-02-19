# [ADR-0019] Backup/Restore Safety with Live SQLite Handles

## Status
Accepted

## Context and Problem Statement
Current backup implementation uses `fs::copy` on a live SQLite database file, which can produce corrupt copies if writes are in progress. Restore overwrites the database file while connections may be open, which does not update the in-memory state of existing libsql connections.

## Decision Drivers
- Data integrity: corrupted backups are worse than no backups
- Operational safety: restore must produce a consistent, usable state
- libsql API constraints: limited access to SQLite Online Backup API
- Hard constraint: local-only operations (remote Turso cannot use file-based backup)

## Considered Options
- `fs::copy` after checkpoint (current)
- `VACUUM INTO` for backup, close-and-reopen for restore
- SQLite Online Backup API (not exposed by libsql)

## Decision Outcome
Chosen option: "VACUUM INTO for backup, close-and-reopen for restore", because VACUUM INTO produces a consistent, standalone copy without shared-lock risks using standard SQL.

### Implementation
- Backup: checkpoint WAL, then execute `VACUUM INTO ?1` with destination path
- Restore: copy backup file over local DB path, then re-initialize schema (caller must create new Persistence instance)
- Document that restore invalidates the current Persistence handle

### Positive Consequences
- Atomic, consistent backup without corruption risk
- Uses standard SQLite semantics available through libsql
- No additional dependencies

### Negative Consequences
- VACUUM INTO is slower than file copy for large databases
- Restore requires re-initialization of the Persistence instance
