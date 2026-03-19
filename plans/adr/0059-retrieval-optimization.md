# ADR-0059: Retrieval Optimization and Benchmark Methodology

## Status
Proposed

## Context
As the concept corpus grows, exact similarity scans become a bottleneck. Baseline benchmarks showed latencies of ~65ms for 200k concepts. Furthermore, benchmark methodology mixed setup costs with steady-state measurements, and concurrency benchmarks didn't measure shared-store contention.

## Decision
1.  **Dense Storage for Hot Path:** Refactor `Singularity` to store concept vectors and indices in contiguous `Vec`s. This avoids `HashMap` lookups and excessive clones during similarity scans.
2.  **Reduced-Candidate Framework:** Introduce a two-stage retrieval pipeline:
    *   Stage 1: Generate a reduced set of candidates using heuristics (e.g., vector bucketing or graph-neighborhood).
    *   Stage 2: Rerank candidates exactly.
3.  **Benchmark Hygiene:**
    *   Separate persistence benchmarks into `cold` (initialization) and `warm` (steady-state).
    *   Implement shared-store concurrency benchmarks with retry logic.
    *   Add realistic (different vectors) and worst-case (identical vectors) retrieval fixtures.

## Consequences
*   **Performance:** ~2.6x speedup for exact scans (~24ms vs ~65ms for 200k concepts).
*   **Scalability:** Bucket-based reduced retrieval provides further speedups (~12ms for 200k concepts).
*   **Observability:** `RetrievalStats` now provides detailed insights into candidate generation and scoring latency.
*   **Stability:** Benchmarks are more reliable and distinguish between setup and operation costs.
