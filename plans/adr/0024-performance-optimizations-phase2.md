# ADR-0024: Performance Optimizations Phase 2

## Status
Deferred (backfilled 2026-04-30)

## Context
Future scalability needs for >200k concepts.

## Decision
Deferred SIMD completion, Product Quantization, and LSH until specific scale triggers are met.

## Resolution (2026-11-25 truth-up, issue #1064)

Two of the three deferred items shipped and their deferral was forgotten in the
plans: SIMD completion landed (`crates/csm-core-lib/src/bundle_simd.rs`,
`hyperdim_simd.rs` — `simd_hypervector_ops: true` in GOALS.md) and LSH landed
behind the opt-in `ann-lsh`/`ann-hnsw` features
(`crates/csm-memory/src/index/lsh.rs`, `hnsw.rs`; scale evidence under
`plans/evidence/`). **Product Quantization remains deferred** — trigger: >200k
concepts with latency degradation; the 10M-concept scale-up is evaluated as
unsupported without it (`ten_million_memory_claim_evaluated: true` in GOAP_STATE).

## Consequences
Maintains code simplicity while identifying clear triggers for future optimization.

## Sources
- Archive: ./plans/.archive/0024-performance-optimizations-phase2.md
