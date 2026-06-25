# ADR-0094: Benchmark Coverage Expansion

## Status

Proposed

## Context and Problem Statement

The `benches/` directory benchmarks: HVec operations, reservoir step, BM25
search, persistence CRUD, bridge retrieval, GraphRAG, and probe at scale.
Missing benchmark coverage for:

1. **Reranking pipeline** (`probe_with_rerankers`) — MMR, recency, and
   cross-encoder reranking are implemented (ADR-0071) but not benchmarked.
   No latency baseline exists for the reranking stage.
2. **Hybrid BM25+HDC retrieval** — Only BM25 is benchmarked in isolation.
   The combined `hybrid_search` path (merge + normalization + weighting)
   has no dedicated benchmark.
3. **Embedding provider encode** — `HdcTextProvider::encode`, projection
   matrix generation, and batch embedding are in the critical path for
   retrieval but lack latency measurements.

## Decision

Add 3 benchmark groups to `benches/`:

### 1. `benches/rerank_benchmark.rs`
- `rerank_mmr_10` / `rerank_mmr_100` — MMR diversity reranking
- `rerank_recency_100` — Recency-weighted reranking
- `rerank_pipeline_combined` — Full pipeline with multiple rerankers

### 2. `benches/hybrid_benchmark.rs`
- `hybrid_search_1k` / `hybrid_search_10k` — End-to-end hybrid search
- `hybrid_merge_normalization` — Isolated merge+normalize step
- `hybrid_weight_selection` — Query-length-dependent weight computation

### 3. `benches/embedding_benchmark.rs`
- `hdc_text_encode_short` / `hdc_text_encode_long` — HdcTextProvider
- `projection_matrix_generate` — One-time projection setup cost
- `batch_embed_100` — Batch encoding throughput

Estimated cost: 5

## Consequences

- Establishes latency baselines for reranking, hybrid search, and embedding.
- Enables performance regression detection in CI benchmark workflow.
- Supports data-driven optimization decisions for the retrieval pipeline.
- Adds ~150 LOC across 3 new bench files.

## References

- `src/retrieval/rerank.rs` — Reranking pipeline (ADR-0071)
- `src/retrieval/hybrid.rs` — Hybrid BM25+HDC search
- `src/embedding/hdc_text.rs` — HDC text embedding provider
- `benches/bm25_benchmark.rs` — Existing BM25-only benchmark
