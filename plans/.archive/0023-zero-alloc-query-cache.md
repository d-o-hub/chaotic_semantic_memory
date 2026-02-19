# [ADR-0023] Zero-Alloc Query Cache Keys and Cached Results

## Status
Accepted

## Context and Problem Statement
`Singularity::find_similar()` includes a query-result cache to speed up repeated probes, but it previously:
- computed cache keys by hashing `query.to_bytes()`, which allocates a `Vec<u8>` per call
- returned cached results by cloning a `Vec<(String, f32)>`, which allocates on cache hits

This conflicts with the crate's performance goals and the "no magic numbers / no hidden runtime costs" constraint in `AGENTS.md`.

## Decision Drivers
- Avoid allocations in hot-path cache lookups for repeated queries.
- Keep existing `find_similar()` API stable for current callers.
- Maintain WASM compatibility and avoid introducing new dependencies.

## Considered Options
- Keep `Vec` cache and clone on hits (status quo).
- Store cached results as `Arc<[(String, f32)]>` and add an `Arc`-returning API for callers that want cache-hit reuse.
- Change `find_similar()` to return `Arc<[(String, f32)]>` (breaking change).

## Decision Outcome
Chosen option: "Store cached results as `Arc<[(String, f32)]>` and add an `Arc`-returning API", because it eliminates cache-hit allocations for the new API while preserving the existing `Vec`-returning API.

### Positive Consequences
- Cache keys hash `HVec10240` words directly (no `to_bytes()` allocation).
- `find_similar_cached()` returns cheap `Arc` clones on cache hits.
- Existing `find_similar()` remains available and delegates to the cached API.

### Negative Consequences
- New API adds surface area (`find_similar_cached`, `probe_batch_cached`).
- Callers using `find_similar()` still allocate to materialize a `Vec` (by design).

## Pros and Cons of the Options

### Keep `Vec` cache and clone on hits
- Good, because no API changes.
- Bad, because cache hits still allocate and key hashing allocates.

### Store cached results as `Arc<[_]>` and add `Arc`-returning API
- Good, because cache hits avoid allocations for callers that can use `Arc`.
- Good, because it avoids breaking changes while enabling higher-performance access patterns.
- Bad, because it adds an additional method and return type to document.

### Change `find_similar()` to return `Arc<[_]>`
- Good, because it forces allocation-free cache hits everywhere.
- Bad, because it is a breaking change for existing callers expecting `Vec`.
