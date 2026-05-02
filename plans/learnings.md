## 2026-05-02 — PR #167: GraphRAG hybrid retrieval (ADR-0070)

### What was fixed
- GraphRAG traversal budget (max_results) correctly excludes the start node.
- GraphRAG traversal allows node revisits to ensure the path with the highest graph score (bottleneck strength / (1 + hops)) is used for ranking.
- Hypervector bundling (HVec10240::bundle) restored fast paths for 1 and 2 vectors (XOR/AND logic).
- Hypervector bundling added a guard (num_vectors >= 32) before spawning Rayon tasks to avoid scheduling overhead for small batches.

### CI jobs fixed
- lint: Fixed float-cmp (use epsilon) and clone-on-copy (remove clone) in tests.
- LOC gate: Satisfied 500-LOC-per-file limit by extracting framework methods to `src/framework_graph_rag.rs` and WASM bindings to `src/wasm_graph_rag.rs`.

### Patterns to remember
- Use extension modules to satisfy strict LOC gates for core framework/WASM files.
- Small batches (e.g. < 32 items) should avoid parallel overhead in hot paths.
- Deterministic graph retrieval requires allowing revisits if a strictly better path strength/score is found later in the traversal.

### Non-fixable issues documented
- 0 items in plans/issues.md

### Skills used
- rust-development: Core implementation and optimization.
- testing-validation: Comprehensive integration tests.
- benchmarking-perf: Measured p50 latency and optimized hot paths.
