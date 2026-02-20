# [ADR-0040] Async Lock Safety: Avoid Holding RwLock Across Await Points

## Status
Accepted

## Context and Problem Statement
Several framework methods hold `tokio::sync::RwLock<Singularity>` write guards across `.await` points (persistence I/O). This is a well-documented Tokio anti-pattern that blocks all other readers/writers for the duration of database operations, causing latency spikes and potential starvation under concurrent access.

Affected methods:
- `ChaoticSemanticFramework::load_replace()` — holds `singularity.write().await` while calling `persistence.load_associations().await` per concept (src/framework.rs:269-291)
- `ChaoticSemanticFramework::load_merge()` — same pattern (src/framework.rs:301-322)
- `import_json()` — holds write lock while calling `persistence.save_concepts().await` and `persistence.save_associations().await` (src/framework_ops.rs:144-169)
- `import_binary()` — same pattern (src/framework_ops.rs:220-246)

## Decision Drivers
- Production systems with concurrent readers will experience high tail latency during load/import
- Tokio documentation explicitly warns against holding locks across yield points
- The fix is straightforward and does not change public API

## Considered Options
- Option A: Restructure lock scopes (collect data, release lock, do I/O, reacquire)
- Option B: Use a separate Mutex for load/import operations (coarser serialization)
- Option C: Accept the current behavior (document as limitation)

## Decision Outcome
Chosen option: "Option A — Restructure lock scopes", because it eliminates the anti-pattern without adding new synchronization primitives and preserves existing concurrency characteristics for all other operations.

### Implementation
For `load_replace`/`load_merge`:
1. Load all concepts from persistence (no lock needed)
2. Acquire write lock, inject concepts, collect concept_ids, release lock
3. Load all associations from persistence (no lock needed)
4. Acquire write lock, apply associations, release lock

For `import_json`/`import_binary`:
1. Acquire write lock, inject concepts + build associations list, release lock
2. Persist concepts and associations (no lock needed)

### Positive Consequences
- Eliminates starvation risk for concurrent probe/inject operations during load/import
- Reduces worst-case latency from O(DB_IO_time) to O(in_memory_inject_time) per lock hold
- Follows established Tokio best practices

### Negative Consequences
- Slightly more complex control flow in affected methods
- During load_replace, a brief window exists between concept injection and association application where queries return results without associations (acceptable for initialization)

## Implementation Status
✅ **Implemented** - All lock scope restructurings completed in Wave 10
