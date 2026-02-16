# [ADR-0001] Use libSQL for Persistence

## Status
Accepted

## Context and Problem Statement
We need a persistence layer for the chaotic semantic memory system that supports:
- Local SQLite for development
- Turso for cloud deployment
- WASM compatibility
- Async/await support

## Decision Drivers
- Must work with Rust 2026 ecosystem
- Must support both local and remote databases
- Must have async support
- Must be actively maintained

## Considered Options
1. **turso-client** - Does not exist as a crate
2. **libsql** - Official Turso Rust client, wraps SQLite with Turso support
3. **rusqlite** - Popular but lacks native Turso support
4. **sqlx** - Great ORM but complex for this use case

## Decision Outcome
Chosen option: **libsql**, because:
- Official Turso client with SQLite compatibility
- Async support built-in
- Works with both local files and remote connections
- Maintained by Turso team

### Positive Consequences
- Single API for local and remote databases
- Good WASM support
- Simple API design

### Negative Consequences
- Smaller community than rusqlite
- Newer crate with potentially fewer examples

## Pros and Cons of the Options

### libsql
* Good: Official Turso support, async, simple API
* Good: Works with local SQLite files
* Bad: Newer crate, smaller community

### rusqlite
* Good: Mature, large community
* Bad: No native Turso support
* Bad: Requires additional work for async

## Links
- [libsql crate](https://crates.io/crates/libsql)
- [Turso documentation](https://docs.turso.tech)