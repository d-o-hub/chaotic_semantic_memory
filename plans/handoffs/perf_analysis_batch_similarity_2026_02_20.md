# Swarm Analysis: Performance Benchmark Issues (Groups B, C)

**Date:** 2026-02-20  
**Run ID:** `swarm_perf_analysis_2026_02_20`  
**Status:** Analysis Complete - Implementation Phase Ready

## Swarm Agents Deployed

| Agent | Role | Status | Output |
|-------|------|--------|--------|
| **@perf** | Performance Analysis | ✅ Complete | Detailed bottleneck analysis |
| **@plan** | GOAP + ADR | ✅ Complete | ADR-0041, GOAP updates |
| **@web-research** | SIMD Techniques | ✅ Complete | std::simd documentation |

---

## Primary Issue: batch_similarity_1000 Performance Gap

### Current State
| Metric | Target | Actual | Gap |
|--------|--------|--------|-----|
| batch_similarity_1000 | <500μs | ~878μs | **76% over target** |

### Root Cause Analysis

**@perf Agent Findings:**

1. **Parallelization Overhead** (Primary Bottleneck)
   - Single similarity: ~200ns
   - 1000 parallel: ~878μs
   - Overhead: **~4.4x** (theoretical: 200μs)
   - Rayon work-stealing + synchronization dominates

2. **SIMD Underutilization**
   - Current: SSE2 (128-bit) per candidate
   - Opportunity: AVX2 (256-bit) batch processing
   - Can process 2 candidates per iteration instead of 1

3. **Memory Access Pattern**
   - Query (1280 bytes) read repeatedly per candidate
   - No prefetching for next candidate
   - Cache line bouncing across threads

### Optimization Strategy (ADR-0041)

**Hybrid Approach: Batched AVX2 + Chunked Parallelism**

```rust
// Level 1: Chunked parallelism
par_chunks(64)  // 64 candidates per thread chunk

// Level 2: Batched AVX2 within each chunk
// Process 2 candidates simultaneously with 256-bit registers
```

**Expected Performance:**
- Baseline: 878μs
- After optimization: **~440μs** (50% improvement)
- Target: <500μs ✅

---

## Files Created/Updated

### New ADR
- ✅ `plans/adr/0041-batch-similarity-optimization.md`

### Updated GOAP
- ✅ `plans/ACTIONS.md` - Added Phase 6B action
- ✅ `plans/GOAP_STATE.md` - Added batch_similarity metrics
- ✅ `plans/ADR_REGISTRY.md` - Registered ADR-0041

---

## Implementation Ready

### Action Details
```yaml
name: optimize_batch_similarity_performance
preconditions:
  benchmarks_exist: true
  simd_hypervector_ops: true
effects:
  batch_similarity_under_500us: true
cost: 4
status: pending
file: src/hyperdim.rs
adr: ADR-0041
```

### Next Steps for Implementation

1. **Spawn @impl agent** to implement ADR-0041
   - Add batched AVX2 function (~30 lines)
   - Implement chunked parallelism (~20 lines)
   - Add tail handling (~10 lines)

2. **Spawn @test agent** to verify
   - Unit tests for batched path
   - Edge cases (empty, single, odd counts)
   - WASM target compilation check

3. **Performance validation**
   ```bash
   cargo bench --bench benchmark -- batch_similarity_1000
   # Target: <500μs median
   ```

---

## Technical Insights

### SIMD Options Researched

| Approach | Gain | Complexity | Portability |
|----------|------|------------|-------------|
| SSE2 (current) | Baseline | Low | Universal |
| AVX2 (chosen) | ~50% | Medium | x86_64 only |
| AVX-512 | ~75% | High | Recent x86_64 |
| NEON | ~40% | Medium | ARM only |
| std::simd | ~45% | Low | Portable* |

*std::simd is nightly-only experimental

### Why Not std::simd?
- Nightly Rust required (not stable)
- Simpler to use existing std::arch intrinsics
- Matches current codebase SIMD approach (ADR-0013)

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Unsafe code bugs | Medium | High | Thorough testing, fallback path |
| LOC budget exceed | Low | Medium | Current 399 → ~440 (within 500) |
| WASM compile break | Low | High | Separate wasm32 cfg path |
| Performance regression | Low | High | Benchmark comparison, rollback plan |

---

## Swarm Coordination Notes

**Group B (Performance) Focus:**
- This is a Group B issue from Wave 5/6 follow-up
- Relates to ADR-0013 (SIMD) and ADR-0007 (Parallel Search)
- Complements reservoir_step_50k optimization (already at 76μs)

**Group C (Observability) Implications:**
- Add metrics for batch_similarity latency
- Track cache hit/miss for batched operations
- Monitor SIMD vs fallback path usage

---

## References

- **ADR-0041**: Batch Similarity Optimization (this analysis)
- **ADR-0013**: SIMD Hypervector Operations (foundation)
- **ADR-0007**: Parallel Similarity Search (Rayon patterns)
- Rust std::simd docs: https://doc.rust-lang.org/std/simd/
- Intel AVX2 Intrinsics: https://www.intel.com/content/www/us/en/docs/intrinsics-guide/

---

## Handoff Contract

**From:** @perf, @plan agents (Analysis Phase)  
**To:** @impl, @test agents (Implementation Phase)

**Deliverables:**
1. ✅ Detailed performance analysis with bottlenecks identified
2. ✅ ADR-0041 with implementation plan
3. ✅ GOAP updates for tracking
4. ✅ Risk assessment and mitigation strategies

**Dependencies:**
- hyperdim.rs current state (399 LOC)
- Existing SIMD infrastructure (SSE2)
- Rayon parallelization patterns

**Validation Criteria:**
- [ ] batch_similarity_1000 < 500μs median
- [ ] Results match individual cosine_similarity
- [ ] WASM target compiles successfully
- [ ] All tests pass

---

**Analysis Complete:** Ready for implementation phase
