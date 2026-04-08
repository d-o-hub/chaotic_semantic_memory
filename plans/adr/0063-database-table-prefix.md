# ADR-0063: Database Table Prefix for Namespace Isolation

## Status
Implemented

## Context

When using a SQLite database that might be shared with other applications or tools,
table names like `concepts`, `associations`, and `concept_versions` can conflict
with tables created by other systems. This causes:
- Schema collision errors when opening a shared database
- Potential data corruption if tables have different structures
- Difficulty identifying which tables belong to CSM

## Decision

Add `csm_` prefix to all database tables:

| Current Name | New Name |
|--------------|----------|
| `concepts` | `csm_concepts` |
| `associations` | `csm_associations` |
| `concept_versions` | `csm_versions` |
| `__schema_version` | `csm_schema_version` |
| `canonical_concepts` | `csm_canonical` |

### Implementation

1. **Schema Migration (v5)**: Add migration to rename existing tables
2. **Update all SQL queries**: Use prefixed table names throughout codebase
3. **Backward compatibility**: Migration handles both new and existing databases

## Module Changes

| Module | Change |
|--------|--------|
| `src/persistence.rs` | Update init_schema with prefixed table names |
| `src/persistence_versions.rs` | Add migration v5 |
| `src/persistence_ops.rs` | Update canonical_concepts queries |
| `src/bridge_persistence.rs` | Update bridge table queries |

## Consequences

### Positive
- Namespace isolation from other applications
- Clear table ownership identification
- Safe coexistence in shared SQLite databases
- Consistent naming convention across all CSM tables

### Negative
- Breaking change for existing databases (requires migration)
- Longer table names in SQL queries
- External tools that query CSM tables must update their queries

## Implementation Notes (2026-04-08)

Implemented in v0.3.0:

- Updated all SQL queries to use `csm_` prefix
- Added schema migration v5 for backward compatibility
- All 21 tests pass with new table names