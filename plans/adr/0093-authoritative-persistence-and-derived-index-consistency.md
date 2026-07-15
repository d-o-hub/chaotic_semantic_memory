# ADR-0093: Authoritative Persistence and Derived ANN Index Consistency

## Status

Proposed (2026-07-14)

## Context and Problem Statement

Concepts and associations are persisted independently from the serialized ANN index. Normal concept mutations update in-memory concepts/indexes and persist concept rows, while ANN bytes are saved only by explicit `persist()`. During `load_replace()` and `load_merge()`, current concept rows are injected first, but any stored `main` ANN blob is then deserialized over the reconstructed index.

The stored index row has `modified_at`, but `load_index()` returns only bytes. There is no authoritative namespace revision, backend/config fingerprint, vector format, or schema version check. Therefore an older snapshot can hide concepts added or removed after the last explicit index persistence.

Related correctness issues share the same missing transaction boundary:

- `inject_concept` and `delete_concept` mutate memory before persistence I/O; a returned error can leave process memory different from durable state.
- `persist`, `load_replace`, and `load_merge` retain state/namespace lock guards across persistence awaits, contrary to ADR-0040.
- association loading performs one query per concept.
- invalid ANN backend configuration reaches `expect()` during lazy namespace creation.

## Decision Drivers

- libSQL rows must remain the durable source of truth.
- A derived ANN artifact must never override newer authoritative rows.
- Public fallible APIs must return `Result`, not panic.
- Tokio state locks must not be held across I/O awaits.
- Probes should remain available while a durable write is in flight.
- Recovery must work for local SQLite and remote Turso without relying on process-local state.

## Considered Options

1. Keep best-effort snapshots and always deserialize when present.
2. Remove ANN persistence and always rebuild.
3. Treat rows as authoritative and persist a revisioned, fingerprinted derived snapshot.
4. Make in-memory state authoritative and asynchronously persist via an outbox.

## Decision Outcome

Chosen option: **rows are authoritative; ANN snapshots are revisioned derivatives; durable mutations are coordinated separately from state locks**.

### Snapshot contract

Persist an envelope containing at least:

- envelope/schema version;
- namespace revision;
- ANN backend and configuration fingerprint;
- vector format/dimension identifier;
- serialized index bytes and optional checksum.

Increment the namespace revision in the same durable transaction as each concept/index-affecting mutation. A load may deserialize the snapshot only when every envelope field matches the authoritative namespace state and configured backend. Missing, corrupt, stale, or incompatible snapshots are discarded and rebuilt from concept rows. Rebuild failure is returned; a stale snapshot is never used as fallback.

`load_merge()` must rebuild from the final union or incrementally preserve that union. It must never replace merged in-memory concepts with a persisted-only snapshot.

### Mutation contract

For persistence-enabled frameworks:

1. Validate inputs and ANN configuration before mutation.
2. Serialize mutations per namespace with a mutation coordinator that is distinct from the singularity/namespace state locks.
3. Commit authoritative rows and namespace revision.
4. Apply the committed mutation to in-memory state under a short state lock.
5. If in-memory application fails after the durable commit, invalidate and reload that namespace from authoritative rows, then return a recovery error with context.
6. Mark any older ANN snapshot stale by revision; snapshot creation occurs after state bytes are copied and state locks are released.

For in-memory-only frameworks, apply the validated mutation directly.

This contract guarantees that a persistence failure does not mutate visible memory. A rare post-commit memory failure is recovered from durable state rather than silently diverging.

### Lock and load contract

- Copy namespace strings, concept snapshots, and serialized index bytes while holding state locks; release guards before `.await`.
- Add one namespace-scoped `load_all_associations` operation rather than one query per concept.
- Validate ANN backend parameters during framework build and propagate construction errors through `Result`.

## Positive Consequences

- Stale ANN data cannot hide current concepts.
- Restart/recovery behavior is deterministic and testable.
- Public configuration errors no longer panic.
- Probes are not blocked by database latency through a state write/read guard.
- Association load query count becomes constant per namespace.

## Negative Consequences

- Namespace revision metadata and migration logic are required.
- Persistence-enabled writes gain coordination and revision-update overhead.
- The durable-first contract may expose committed data to another instance just before the current instance updates its cache; the coordinator/reload path must document this short window.
- Snapshot compatibility requires explicit version maintenance.

## Pros and Cons of the Options

### Keep best-effort snapshots

- Good, because it requires no migration.
- Bad, because it preserves the verified stale-overwrite defect.

### Always rebuild

- Good, because correctness is simple.
- Good, because no snapshot migration is needed.
- Bad, because large-index startup cost becomes unavoidable.

### Revisioned derived snapshot

- Good, because startup acceleration is retained without sacrificing authority.
- Good, because corruption/backend changes become detectable.
- Bad, because revisions and fingerprints add implementation complexity.

### In-memory authority with outbox

- Good, because writes can return quickly.
- Bad, because crash consistency, retries, ordering, and multi-instance semantics become substantially more complex than current requirements justify.

## TRIZ Rationale

- **Segmentation:** separate authoritative rows from derived search acceleration.
- **Separation in time:** complete durable mutation before publishing new in-memory state; serialize snapshot bytes before I/O.
- **Intermediary:** a namespace mutation coordinator provides ordering without holding query state locks across awaits.

## Follow-up Actions

- `fix_ann_backend_validation`
- `enforce_authoritative_persistence_and_ann_revision`
- `bulk_load_associations_and_release_state_locks`
- Add stale insert/update/delete, corrupt snapshot, backend mismatch, HNSW/LSH round-trip, merge-union, and concurrent starvation regression tests.
- Benchmark revision overhead, rebuild cost, and load query count.

## Acceptance Criteria

- Invalid HNSW/LSH configuration returns `MemoryError::InvalidInput` from build; no production `expect` is involved.
- A snapshot saved before insert/update/delete is rejected on next load and all authoritative concepts remain searchable.
- Corrupt or backend-mismatched snapshots rebuild successfully from rows.
- `load_merge` preserves pre-existing plus persisted concepts.
- Persistence failure leaves visible memory unchanged.
- No singularity or namespace state lock guard spans a persistence await.
- Association loading uses one namespace query for 1, 1,000, and 10,000 concepts.
- A bounded concurrent inject/probe/persist/load test completes without deadlock or starvation.
