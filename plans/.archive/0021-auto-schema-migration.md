# [ADR-0021] Automatic Schema Migration on Startup

## Status
Accepted

## Context and Problem Statement
`apply_migrations` exists in persistence_ops.rs but is never called automatically. The schema version is tracked (v1 baseline) but new migrations are not applied on startup. Operators must manually discover and call the migration API.

## Decision Drivers
- Operational simplicity: zero-touch upgrades for schema changes
- Data integrity: schema must match code expectations
- Safety: migrations must be idempotent and forward-only

## Considered Options
- Manual migration call by operator (current)
- Auto-migrate during `init_schema()`
- Auto-migrate with dry-run/preview option

## Decision Outcome
Chosen option: "Auto-migrate during init_schema()", because it provides zero-touch upgrades with minimal complexity.

### Implementation
- Add `const LATEST_SCHEMA_VERSION: i64` in persistence module
- Call `apply_migrations(LATEST_SCHEMA_VERSION)` at the end of `init_schema()`
- Migrations are forward-only; rollback requires restore from backup
- Log migration steps at info level

### Positive Consequences
- Zero-touch schema upgrades across versions
- Consistent schema state across all database instances
- Schema version always matches code expectations

### Negative Consequences
- No built-in rollback (mitigated by backup-before-upgrade pattern)
- Auto-migration on startup adds latency for first connection
