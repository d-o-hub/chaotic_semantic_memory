# ADR-0021: Auto Schema Migration

## Status

Accepted (backfilled 2026-05-01)

## Context

Database schema evolves over versions:
- Original: Manual migration required
- Problem: Version mismatch causes errors
- Problem: Users forget to migrate

## Decision

Implement **auto-migration on connection**.

**Rationale:**
- Schema versioning with __schema_version table
- Migration runner: apply_migrations(current, target)
- Versioned migrations in migrations/
- Auto-apply on Persistence::new_*()

## Consequences

### Positive
- Automatic schema updates
- No manual migration steps
- Version compatibility enforced
- Rollback support for failed migrations

### Negative
- Migration overhead on first connection
- Complex migrations may fail
- Schema version tracking required

## Implementation

- Module: `src/persistence.rs`
- Schema: csm_schema_version table (v5 prefix)
- Migrations: numbered SQL files
- Auto-apply: on new_local/new_turso

## Sources

- ACTIONS.md lines 739-752 (add_schema_migration_support action)
- MEMORY.md: "Migrations are auto-applied on Persistence::new_*"
- src/persistence.rs: migration runner