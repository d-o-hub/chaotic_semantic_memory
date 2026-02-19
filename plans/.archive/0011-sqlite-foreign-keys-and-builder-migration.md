# [ADR-0011] SQLite Foreign Keys and libsql Builder Migration

## Status
Accepted

## Context and Problem Statement
`Persistence` currently relies on per-operation connections. Two issues remained:
- Deprecated `libsql::Database::open/open_remote` constructors were still used.
- SQLite foreign-key constraints were defined in schema but not guaranteed active per connection.

## Decision Drivers
- Keep `libsql` as the only database client.
- Remove deprecated API usage.
- Enforce relational integrity for concept-association links.
- Preserve async Tokio-compatible persistence APIs.

## Considered Options
1. Keep deprecated constructors and manual cleanup.
2. Move to `libsql::Builder` and enable foreign keys per connection.
3. Add a connection pool and one-time PRAGMA setup.

## Decision Outcome
Chosen option: **Move to `libsql::Builder` and enable foreign keys per connection**.

### Positive Consequences
- Removes deprecated API suppression and aligns with `libsql` guidance.
- Ensures FK constraints are active even with per-operation connections.
- Keeps persistence model simple and thread-safe.

### Negative Consequences
- Adds one extra statement (`PRAGMA foreign_keys=ON`) on each connection.
- Remote backends may pay minor overhead for the additional command.

## Pros and Cons of the Options

### Keep deprecated constructors
- Good, because no code changes are required.
- Bad, because deprecations accumulate and hide migration debt.

### Builder + per-connection PRAGMA
- Good, because APIs are current and constraints are always enforced.
- Good, because behavior is explicit and testable.
- Bad, because each operation has slight setup overhead.

### Pooling
- Good, because PRAGMA cost can be amortized.
- Bad, because it adds complexity not currently justified.
