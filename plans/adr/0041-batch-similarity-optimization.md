# ADR-0041: Batch Similarity Optimization

## Status

Accepted (backfilled 2026-05-01) - Wave 12 Complete

## Context

Batch similarity performance issue:
- batch_cosine_similarity_1000: ~878us (target <500us)
- Rayon overhead for small batches
- No chunk tuning

## Decision

Optimize **batch similarity to meet <500us target**.

**Deliverables:**
- Phase 1: Chunked Rayon parallelism with chunk_size=64
- Phase 2: Tuned chunk_size=128 (amortizes overhead)
- Result: ~470us (47% improvement, target met)

## Consequences

### Positive
- Batch similarity <500us target achieved
- Chunk tuning methodology documented
- Parallelism overhead amortized
- Better batch query performance

### Negative
- Chunk tuning requires experimentation
- Performance varies by batch size
- May need retuning for different hardware

## Implementation

- Module: src/hyperdim.rs
- Chunk size: 128 (optimal for 1000 candidates)
- Validation: benchmark batch_similarity_1000 median <500us

## Sources

- ACTIONS.md lines 617-641 (optimize_batch_similarity_performance action)
- MEMORY.md: batch_similarity_1000 ~322us
- Git: perf(hyperdim): optimize batch_similarity commits