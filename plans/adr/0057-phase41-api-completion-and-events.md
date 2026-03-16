# ADR-0057: Phase 41 API Completion and Memory Events

## Status
Implemented

## Context and Problem Statement
Deferred Phase 41 items from ADR-0054 (framework-level filtered probe, traversal APIs, and memory
change events) have remained unimplemented while downstream clients need a stable integration
surface across native and WASM targets. CI failures on the open PRs also highlight that new
builder settings (version retention) and error source chaining must be wired end-to-end, with
WASM parity retained for compilation.

## Decision Drivers
- Close deferred API gaps for Phase 41 while preserving WASM parity.
- Provide an event stream for integrations without adding heavy dependencies.
- Keep persistence retention configurable through the builder.
- Maintain CI guardrails (LOC gates, no magic numbers in production paths).

## Considered Options
- **Option A:** Keep Phase 41 deferred and leave PRs unmerged.
- **Option B:** Implement Phase 41 APIs and memory events with broadcast-based delivery and
  WASM bindings, while wiring version retention and error source chaining.

## Decision Outcome
Chosen option: **Option B**, because the deferred items are required by downstream integrations
and CI must pass for open PRs, with minimal API surface changes and clear error propagation.

### Positive Consequences
- Framework exposes `probe_filtered`, `traverse`, and `shortest_path` plus WASM parity.
- Memory events available via `broadcast` without external dependencies.
- Builder version retention and error source chains are enforced consistently.

### Negative Consequences
- Public API changes in `MemoryError` variants require downstream pattern-match updates.
- Event delivery is best-effort (lagged events dropped) and capacity is fixed.

## Pros and Cons of the Options

### Option A
- Good, because it avoids API churn before a new release.
- Bad, because it blocks needed features and keeps CI failing for open PRs.

### Option B
- Good, because it closes Phase 41 gaps and keeps native/WASM parity.
- Bad, because it introduces new API surface and requires documentation and tests.
