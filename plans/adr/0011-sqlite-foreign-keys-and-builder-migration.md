# ADR-0011: SQLite Foreign Keys & Builder Migration

## Status

Accepted (backfilled 2026-05-01)

## Context

Database integrity requires foreign key constraints:
- Original: SQLite foreign_keys pragma disabled by default
- Problem: Associations can reference non-existent concepts
- Problem: libSQL deprecated Database::open/open_remote constructors

## Decision

**Enable foreign_keys pragma** and migrate to libsql::Builder API.

**Rationale:**
- PRAGMA foreign_keys=ON for every connection
- Ensures associations have valid concept references
- Prevents orphaned associations on concept deletion
- Builder pattern is current libSQL API

## Consequences

### Positive
- Database integrity enforced at SQLite level
- No orphaned associations possible
- Builder API is future-proof
- Tests validate constraint enforcement

### Negative
- Insert order matters (concept before association)
- Cascade deletion required for concept removal
- Builder API more verbose than deprecated constructors

## Implementation

- Module: `src/persistence.rs`
- Connection: PRAGMA foreign_keys=ON on every connect()
- Migration: libsql::Builder::new_local/new_remote
- Tests: persistence_roundtrip.rs validates constraints

## Sources

- ACTIONS.md lines 230-267 (enforce_sqlite_foreign_keys, migrate_libsql_builder_api)
- src/persistence.rs: foreign_keys pragma
- tests/persistence_roundtrip.rs