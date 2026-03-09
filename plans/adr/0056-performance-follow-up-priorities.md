# [ADR-0056] Performance Follow-up Priorities After v0.2.0

## Status
Implemented

## Context and Problem Statement

An optimization review against the current codebase shows that several suggested changes are
already implemented or do not have clear near-term impact:

- `src/reservoir.rs` already stores reservoir weights as `f32`
- `src/reservoir.rs` already uses sparse row storage for `w_in` and `w_res`
- spectral radius is estimated during initialization and `set_spectral_radius()`, not per step
- `src/persistence.rs` and `src/persistence_ops.rs` already batch writes inside explicit transactions
- `Cargo.toml` already enables `wasm-opt` for release builds

The remaining codebase-backed gaps are narrower:

1. `src/singularity.rs::find_similar_cached()` still performs an exact O(n) scan and first
   materializes `Vec<(String, HVec10240)>` on every cache miss. That adds allocation and cloning
   cost before similarity work even starts.
2. `src/persistence.rs` uses `PRAGMA wal_checkpoint(TRUNCATE)` but local SQLite initialization does
   not explicitly set `PRAGMA journal_mode=WAL`, leaving local concurrent read/write performance to
   backend defaults rather than crate policy.

The current performance gate still passes for the reservoir hot path (`reservoir_step_under_100us`
is true), so the next planning step should focus on the code paths that still show direct impact in
the current implementation.

## Decision Drivers

- Favor changes justified by the current code, not by generic optimization lists
- Preserve exact search semantics unless scale data shows they are insufficient
- Keep architecture changes proportional to measured benefit
- Respect `AGENTS.md` constraints: no unused code, Tokio for I/O, Rayon only off `wasm32`
- Avoid adding risky dependencies or FFI without evidence they are required

## Considered Options

- Option A: Implement the full recommendation set now (`rkyv`, intESN quantization, LSH, extra SIMD,
  allocator changes, prepared statement caching)
- Option B: Prioritize only codebase-proven follow-up work now: probe scan layout cleanup and local
  SQLite WAL policy; keep ANN/LSH conditional on scale-triggered evidence
- Option C: Make no planning changes because current benchmarks pass

## Decision Outcome

Chosen option: "Option B", because it targets the remaining hotspots that are visible in the code
today without committing the crate to unnecessary architectural churn.

### Workstream 1: Exact Probe Path Cleanup

- Refactor similarity search so cache misses do not first clone all concept IDs and vectors into a
  temporary `Vec`
- Add probe benchmarks at larger concept counts to determine when exact scan stops being acceptable
- Keep ANN/LSH as an optional follow-on only after measured degradation at the existing deferred
  trigger (`>200k` concepts with latency issues)

### Workstream 2: Local SQLite WAL Policy

- Explicitly enable WAL mode for local SQLite initialization
- Keep the remote Turso path unchanged
- Add tests that verify `journal_mode=WAL` and checkpoint compatibility

### Positive Consequences

- Planning stays aligned with real code hotspots
- Avoids premature complexity from quantization, zero-copy archival, or FFI kernels
- Makes the deferred LSH work conditional on benchmark evidence instead of assumption
- Brings local SQLite behavior in line with existing checkpoint usage

### Negative Consequences

- Probe remains exact O(n) until the follow-up work lands
- Large-scale ANN search remains deferred until new benchmarks justify it
- Export/import stays on `serde`/`bincode`, which may remain suboptimal for very large snapshots

## Pros and Cons of the Options

### Option A

- Good, because it aggressively chases theoretical maximum throughput
- Bad, because most of the proposed work is not justified by the current implementation
- Bad, because it adds significant maintenance and testing burden across reservoir, WASM, and
  persistence paths

### Option B

- Good, because it focuses on the two remaining gaps that are directly visible in the code
- Good, because it keeps exact semantics and avoids architectural churn until scale data demands it
- Bad, because it does not immediately solve very large concept-store search latency

### Option C

- Good, because it avoids any extra work while current gates pass
- Bad, because it leaves avoidable probe allocations and undefined local WAL policy in place

## Follow-up Actions

1. Completed: removed per-query probe materialization and added exact-scan benchmark coverage for
   10k, 100k, and 200k concept scales.
2. Completed: enabled WAL mode for local SQLite initialization and validated WAL + checkpoint
   compatibility in integration tests.
3. Ongoing (deferred trigger): revisit optional ANN/LSH indexing only when scale benchmarks show
   evidence beyond the current threshold policy.

## Implementation Notes (2026-03-09)

- `src/singularity.rs`: cache-miss probe path no longer clones all concepts into an intermediate
  vector before similarity computation.
- `src/persistence.rs`: local connections now run `PRAGMA journal_mode=WAL;` and all connections
  continue enforcing `PRAGMA foreign_keys=ON;`.
- `tests/persistence_crud.rs`, `tests/performance_targets.rs`: added explicit WAL-mode assertions.
- `benches/benchmark.rs`: added `probe_exact_scan_scale` benchmark group (10k/100k/200k).
- `plans/GOAP_STATE.md` and `plans/ACTIONS.md` updated to mark Phase 48 complete.
