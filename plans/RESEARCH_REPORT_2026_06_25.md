# Research Report: 2026-01-01 → 2026-06-25

## Papers Found (9 HIGH impact)

| # | Paper | Year | Technique | Integration |
|---|-------|------|-----------|-------------|
| 1 | AFRICO (Lupascu & Coca, Sci Rep) | 2026 | EKF-adaptive input/feedback + sparse OFR readout | reservoir.rs |
| 2 | EST (Bendi-Ouis et al.) | 2026 | Multi-reservoir working memory with trained dynamics | reservoir.rs |
| 3 | PAG (arXiv:2603.06660) | 2026 | Projection-augmented graph, asymmetric comparison | index/ |
| 4 | AQR-HNSW (arXiv:2602.21600) | 2026 | Density-aware quantization + multi-stage reranking | index/hnsw.rs |
| 5 | ASH (Tepper & Willke) | 2026 | Learned projection + asymmetric scalar quantization | hyperdim_simd.rs |
| 6 | Ultra-Quantisation (arXiv:2506.00528) | 2026 | 1.58-bit ternary {-1,0,1} encoding | hyperdim_binary.rs |
| 7 | QuIVer (Xiao et al.) | 2026 | BQ-native graph topology, training-free | index/ |
| 8 | Dynamic Query Mod (arXiv:2605.23807) | 2026 | Query center-point estimation for LSH | index/lsh.rs |
| 9 | 2DLCHM Hashing (Nature) | 2025 | 2D hyperchaotic hash with parallel feedback | encoder.rs |

## Implementation Added
- `csm-core/src/hyperdim_ternary.rs` — TernaryHVec (1.58-bit encoding)
- `csm-core/src/chaos_2dlchm.rs` — 2DLCHM hyperchaotic PRNG
- `csm-core/src/reservoir_africo.rs` — AFRICO adaptive training
- `csm-memory/src/index/lsh_query_mod.rs` — Dynamic query modification
- `benches/ternary_quantization_benchmark.rs` — Criterion benchmarks
