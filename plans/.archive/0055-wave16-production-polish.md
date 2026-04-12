# [ADR-0055] Wave 16: Production Polish & Correctness

## Status
Implemented

## Context and Problem Statement

After 15 waves of development, the crate is production-ready with all validation gates passing. However, deep analysis reveals several correctness, safety, and quality issues that should be resolved before v0.2.0:

1. **Panic paths in production code**: `BundleAccumulator::remove` uses `assert!` (panics), `Reservoir::to_hypervector` uses `unwrap_or_else(|_| panic!(...))`.
2. **Documentation/code mismatch**: `shortest_path` docs claim weighted Dijkstra (`-ln(strength)`) but implementation is unweighted BFS.
3. **Hash stability**: `TextEncoder` docs say "FNV-1a" but uses `DefaultHasher` (SipHash), which is not guaranteed stable across Rust versions.
4. **WASM parity gaps**: Missing wrappers for `update_concept_metadata`, `clear_associations`, graph traversal APIs, and lossy metadata round-tripping.
5. **Missing test coverage**: No tests for `TextEncoder` regression vectors, graph traversal edge cases (cycles), `BundleAccumulator` edge cases.
6. **Missing benchmarks**: No criterion benchmarks for `TextEncoder::encode`, `find_similar_filtered`, or graph traversal operations.

## Decision Drivers
- No panic paths in production code (crate constraint)
- Documentation must match behavior exactly
- Determinism is foundational for an AI memory system
- WASM parity enables browser deployment
- Performance gates require benchmark coverage for new hot paths

## Considered Options

### Option A: Docs-only fixes + panic removal (minimal)
Fix documentation, remove panics, skip WASM and benchmarks.

### Option B: Full production polish (recommended)
Fix all 6 categories: panics, docs/code mismatch, hash stability, WASM parity, tests, benchmarks.

### Option C: Option B + weighted Dijkstra shortest_path
Also implement actual weighted shortest path algorithm.

## Decision Outcome

Chosen option: **Option C**, because:
- Panics violate project hard constraints
- Doc/code mismatch is a correctness bug that misleads users
- Hash instability risks silent breaking changes on Rust upgrades
- WASM parity gaps block real browser adoption
- Weighted shortest_path was the documented intent and adds real value for knowledge graphs

### Positive Consequences
- Zero panic paths in non-WASM production code
- All documentation matches implementation
- Deterministic encoding guaranteed across Rust versions
- WASM feature parity for graph/metadata operations
- Benchmark coverage for all new hot paths from Wave 15

### Negative Consequences
- Hash change in TextEncoder is a breaking change for persisted data (mitigated by configurable hash)
- Dijkstra adds complexity to graph_traversal.rs (~40 LOC)
- WASM binary size may increase slightly with new wrappers

## Implementation Plan

### Phase 42: Panic Path Elimination (cost: 3)
- `BundleAccumulator::remove` → `try_remove() -> Result<()>` + keep `remove` as no-op in release
- `Reservoir::to_hypervector` → replace panic with `MemoryError::Reservoir`
- `TextEncoder::encode` → add `bundle_or_zero` helper, document intentional fallback

### Phase 43: Correctness Fixes (cost: 5)
- `TextEncoder::stable_hash` → implement actual FNV-1a (tiny, no deps)
- Add `TextEncoderConfig::hash_algorithm` field for backward compat
- `shortest_path` → implement weighted Dijkstra with `-ln(strength)` cost
- Add `shortest_path_hops` for the current BFS behavior (backward compat)
- Add golden regression test vectors for TextEncoder

### Phase 44: WASM Parity (cost: 4)
- Add `update_concept_metadata` WASM wrapper
- Add `clear_associations` WASM wrapper
- Add `neighbors`, `bfs`, `shortest_path` WASM wrappers
- Fix metadata JSON parsing (use `js_sys::JSON::parse` for type fidelity)

### Phase 45: Test Coverage (cost: 4)
- TextEncoder: golden vector regression tests (known input → known hash → known HVec)
- Graph traversal: cycle detection test, disconnected graph test, max_results limit test
- BundleAccumulator: remove from empty (no panic), remove more than added
- find_similar_filtered: empty filter, no-match filter, large dataset

### Phase 46: Benchmark Coverage (cost: 3)
- TextEncoder::encode (short/medium/long text)
- find_similar_filtered (100/1k/10k concepts with filters)
- BFS and shortest_path (sparse/dense graphs)
- BundleAccumulator add/remove/finalize cycle

### Phase 47: Documentation Refresh (cost: 2)
- Update CHANGELOG.md for v0.2.0 changes
- Update README.md with TextEncoder and graph traversal examples
- Add encoder.rs and graph_traversal.rs to book/src/
- Update llms.txt and llms-full.txt

## Pros and Cons of the Options

### Option A: Minimal
- Good, because lowest effort (~1 day)
- Bad, because leaves WASM gaps and no benchmarks for new features

### Option B: Full polish
- Good, because comprehensive quality improvement
- Good, because all new Wave 15 features get proper coverage
- Bad, because ~2-3 days effort

### Option C: Full polish + Dijkstra (chosen)
- Good, because fulfills documented API contract
- Good, because weighted paths are genuinely useful for knowledge graphs
- Bad, because slightly more complex than BFS-only
- Neutral, because `shortest_path_hops` preserves backward compat
