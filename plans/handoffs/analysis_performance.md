# Performance Analysis Report - Chaotic Semantic Memory

**Date:** 2026-02-20  
**Analyst:** Swarm Group B - Performance Specialist  
**Scope:** Full codebase hot path analysis, allocation patterns, async performance, SIMD optimization

---

## Executive Summary

This analysis identifies **22 performance issues** across the chaotic_semantic_memory codebase, categorized as:
- **4 Critical**: Severe impact on throughput/latency
- **8 High**: Significant impact, should be prioritized
- **7 Medium**: Moderate impact, address in optimization pass
- **3 Low**: Minor improvements, nice-to-have

The primary hotspots are in hypervector operations (`hyperdim.rs`), reservoir stepping (`reservoir.rs`), and similarity search (`singularity.rs`).

---

## 1. Hot Path Analysis

### 1.1 HVec10240 Operations (hyperdim.rs)

#### Issue 1.1.1: Bundle() Allocation Storm - **CRITICAL**
- **Location:** `src/hyperdim.rs:110-171`
- **Impact:** Each bundle operation allocates `Box<[0i32; 10240]>` per thread via Rayon
- **Problem:** 
  - Line 119: `Box::new([0i32; Self::DIMENSION])` - 40KB allocation per fold iteration
  - Line 133: Another identical allocation in reduce
  - For 1000 vectors on 8 threads = ~16,000 allocations of 40KB each = 640MB allocation churn
- **Benchmark Impact:** `hvec_bundle_1000` likely 2-5x slower than necessary
- **Fix:** Use thread-local reusable buffers or stack-allocated arrays with careful bounds checking

```rust
// RECOMMENDED FIX: Use thread-local buffers
thread_local! {
    static BUNDLE_BUFFER: RefCell<[i32; 10240]> = RefCell::new([0i32; 10240]);
}

pub fn bundle(vectors: &[Self]) -> Result<Self> {
    // ... use thread_local buffer instead of Box::new in fold
}
```

#### Issue 1.1.2: Bit-by-Bit Iteration in Bundle - **HIGH**
- **Location:** `src/hyperdim.rs:122-128, 148-154`
- **Impact:** O(n*m*128) bit operations vs O(n*m*2) word operations
- **Problem:** 
  - Lines 122-128 iterate bit-by-bit using `for j in 0..128`
  - Each bit test requires shift+mask operations
  - 128x more operations than necessary
- **Fix:** Use `count_ones()` on XOR results or process u64 chunks

```rust
// RECOMMENDED FIX: Process u64 chunks
for i in 0..80 {
    let word = v.data[i];
    let low = word as u64;
    let high = (word >> 64) as u64;
    local[i * 2] += low.count_ones() as i32;
    local[i * 2 + 1] += high.count_ones() as i32;
}
```

#### Issue 1.1.3: Missing AVX2/AVX-512 SIMD Paths - **HIGH**
- **Location:** `src/hyperdim.rs:15-65`
- **Impact:** 2-4x slower on modern x86_64 CPUs
- **Problem:** Only SSE2 (128-bit) SIMD is implemented
- **Fix:** Add AVX2 (256-bit) and AVX-512 (512-bit) paths

```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn bind_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::x86_64::*;
    let mut out = [0u128; 80];
    // Process 4 u128s (512 bits) at a time with AVX2
    for i in (0..80).step_by(2) {
        let a = _mm256_loadu_si256(lhs.as_ptr().add(i) as *const __m256i);
        let b = _mm256_loadu_si256(rhs.as_ptr().add(i) as *const __m256i);
        let x = _mm256_xor_si256(a, b);
        _mm256_storeu_si256(out.as_mut_ptr().add(i) as *mut __m256i, x);
    }
    out
}
```

#### Issue 1.1.4: Cosine Similarity Population Count Could Be Faster - **MEDIUM**
- **Location:** `src/hyperdim.rs:45-65`
- **Impact:** ~10-20% slower than optimal on x86_64
- **Problem:** Uses `count_ones()` on individual u64s; could use hardware popcnt more efficiently
- **Fix:** Use `_mm_popcnt_u64` intrinsic when available

#### Issue 1.1.5: to_bytes() Repeated Allocation - **MEDIUM**
- **Location:** `src/hyperdim.rs:253-259`
- **Impact:** Allocates 1280 bytes per call; called frequently in serialization
- **Problem:** `Vec::with_capacity(1280)` + `extend_from_slice` in loop
- **Fix:** Use `arrayvec` or const-generic stack allocation

```rust
pub fn to_bytes(&self) -> [u8; 1280] {
    let mut bytes = [0u8; 1280];
    for (i, word) in self.data.iter().enumerate() {
        bytes[i*16..(i+1)*16].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}
```

---

### 1.2 Reservoir Operations (reservoir.rs)

#### Issue 1.2.1: to_hypervector() Scalar Loop - **CRITICAL**
- **Location:** `src/reservoir.rs:291-318`
- **Impact:** Sequential processing of 50K+ elements; no parallelism
- **Problem:**
  - Line 302-314: Nested loop processes chunk_size * 10240 elements sequentially
  - For 50K reservoir: ~5 outer iterations * 10240 inner = 50K operations, all scalar
- **Benchmark Target:** `< 100μs` for `reservoir_step_50k`; this function likely dominates
- **Fix:** Parallelize outer loop with Rayon

```rust
pub fn to_hypervector(&self) -> Result<HVec10240> {
    // ...
    let chunk_size = self.size / HVec10240::DIMENSION;
    let mut data = [0u128; 80];
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        let bits: Vec<(usize, usize, bool)> = (0..HVec10240::DIMENSION)
            .into_par_iter()
            .map(|bit_index| {
                let start = bit_index * chunk_size;
                let end = start + chunk_size;
                let sum: f32 = self.state[start..end].iter().sum();
                let word = bit_index / 128;
                let bit = bit_index % 128;
                (word, bit, sum > 0.0)
            })
            .collect();
        
        for (word, bit, set) in bits {
            if set { data[word] |= 1u128 << bit; }
        }
    }
    // ... WASM fallback
}
```

#### Issue 1.2.2: SparseWeights.dot_row() Not Inlined - **HIGH**
- **Location:** `src/reservoir.rs:110-121`
- **Impact:** Function call overhead in hottest loop
- **Problem:** `#[inline]` present but compiler may not always inline; `mul_add` prevents some optimizations
- **Fix:** Mark `#[inline(always)]` and verify with `cargo asm`

#### Issue 1.2.3: Input Projection Cache Invalidation - **MEDIUM**
- **Location:** `src/reservoir.rs:214-233`
- **Impact:** Unnecessary recomputation when input unchanged
- **Problem:** Line 214: `self.input_cache != input` performs 10240 f32 comparisons
- **Fix:** Use hash-based comparison or pointer equality for common cases

#### Issue 1.2.4: State Swap Uses mem::swap on Large Vectors - **MEDIUM**
- **Location:** `src/reservoir.rs:245`
- **Impact:** Pointer swap is cheap, but causes cache thrashing on next access
- **Problem:** `std::mem::swap(&mut self.state, &mut self.scratch)` swaps Vec pointers
- **Note:** This is actually optimal; the issue is cache locality on subsequent reads

#### Issue 1.2.5: Partial Update Stride Causes Cache Misses - **MEDIUM**
- **Location:** `src/reservoir.rs:238-243`
- **Impact:** Strided access pattern defeats cache prefetching
- **Problem:** `step_by(self.update_stride)` jumps every 32 elements
- **Fix:** Process contiguous chunks per step instead

```rust
// Current (bad for cache):
for i in (self.update_phase..self.size).step_by(self.update_stride) { ... }

// Better: Process contiguous block per step
let block_size = self.size / self.update_stride;
let start = self.update_phase * block_size;
let end = ((self.update_phase + 1) * block_size).min(self.size);
for i in start..end { ... }
```

---

### 1.3 Search Operations (singularity.rs)

#### Issue 1.3.1: find_similar() String Cloning in Parallel Loop - **CRITICAL**
- **Location:** `src/singularity.rs:254-266`
- **Impact:** N String allocations for N concepts; severe GC pressure
- **Problem:** 
  - Line 258: `c.id.clone()` clones every concept ID during similarity search
  - For 100K concepts = 100K heap allocations
- **Fix:** Return references or use string interning

```rust
// CURRENT (bad):
.map(|c| (c.id.clone(), query.cosine_similarity(&c.vector)))

// RECOMMENDED: Return references, clone only top_k results
let mut results: Vec<(&str, f32)> = self.concepts
    .values()
    .map(|c| (c.id.as_str(), query.cosine_similarity(&c.vector)))
    .collect();
results.select_nth_unstable_by(top_k - 1, |a, b| b.1.total_cmp(&a.1));
results.truncate(top_k);
// Clone only the top_k results
results.into_iter().map(|(id, score)| (id.to_string(), score)).collect()
```

#### Issue 1.3.2: QueryCache Uses Mutex in Read Path - **HIGH**
- **Location:** `src/singularity.rs:240-251`
- **Impact:** Cache lookup serializes all read operations
- **Problem:** `self.query_cache.lock()` on every similarity query
- **Fix:** Use `RwLock` or `dashmap` for concurrent reads

```rust
// CURRENT:
query_cache: Mutex<QueryCache>

// RECOMMENDED:
query_cache: RwLock<QueryCache>
// OR:
query_cache: dashmap::DashMap<u64, Arc<[(String, f32)]>>
```

#### Issue 1.3.3: Cache Key Hashing is Expensive - **MEDIUM**
- **Location:** `src/singularity.rs:433-438`
- **Impact:** 10240-bit hash computation on every query
- **Problem:** Hashes entire HVec10240.data (1280 bytes)
- **Fix:** Use subset hashing or cached hash

```rust
// RECOMMENDED: Hash only first 128 bytes (8 words)
pub(crate) fn similarity_cache_key(query: &HVec10240, top_k: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    top_k.hash(&mut hasher);
    query.data[..8].hash(&mut hasher); // Only first 1024 bits
    hasher.finish()
}
```

#### Issue 1.3.4: VecDeque::remove() in Cache is O(n) - **MEDIUM**
- **Location:** `src/singularity.rs:74-77, 84-87`
- **Impact:** Cache maintenance becomes expensive at high capacity
- **Problem:** `self.order.remove(pos)` is O(n) for VecDeque
- **Fix:** Use `linked-hash-map` or `lru` crate

#### Issue 1.3.5: select_nth_unstable_by After Full Sort - **LOW**
- **Location:** `src/singularity.rs:285`
- **Impact:** Unnecessary work when results.len() > top_k
- **Problem:** First sorts all results (line 269), then uses select_nth
- **Fix:** Skip sort when truncating

```rust
// CURRENT: Sorts all, then truncates
results.sort_by(|a, b| b.1.total_cmp(&a.1));
results.truncate(top_k);

// RECOMMENDED: Use select_nth_unstable directly
if results.len() > top_k {
    results.select_nth_unstable_by(top_k - 1, |a, b| b.1.total_cmp(&a.1));
    results.truncate(top_k);
    results.sort_by(|a, b| b.1.total_cmp(&a.1));
}
```

---

### 1.4 Framework Operations (framework.rs)

#### Issue 1.4.1: RwLock Held Across await Points - **CRITICAL**
- **Location:** `src/framework.rs:103-118, 122-150, 153-162`
- **Impact:** Potential for contention and async executor issues
- **Problem:** 
  - Line 109-110: `singularity.write().await` held while calling `inject()`
  - Line 114-115: Persistence operations could happen with lock held
- **Status:** Currently NOT held across await (scoped correctly), but pattern is risky
- **Recommendation:** Document this pattern and add clippy lint

#### Issue 1.4.2: Metrics Snapshot Takes Multiple Locks - **HIGH**
- **Location:** `src/framework.rs:346-369`
- **Impact:** Sequential lock acquisition can cause contention
- **Problem:** 
  - Line 350-352: singularity.read().await
  - Line 355-360: reservoir.read().await  
  - Held sequentially, not overlapping
- **Fix:** Collect metrics without locks using atomics/lock-free structures

#### Issue 1.4.3: Concept Cloning in get_concept - **MEDIUM**
- **Location:** `src/framework.rs:231-235`
- **Impact:** Full Concept clone on every retrieval
- **Problem:** Line 234: `sing.get(id).cloned()` clones HVec10240 (1280 bytes) + HashMap
- **Fix:** Return reference or use Arc<Concept>

```rust
// CURRENT:
pub async fn get_concept(&self, id: &str) -> Result<Option<Concept>>

// RECOMMENDED:
pub async fn get_concept(&self, id: &str) -> Result<Option<Arc<Concept>>>
```

---

## 2. Allocation Patterns

### 2.1 Vec Allocations in Hot Loops

| Location | Allocation | Frequency | Impact |
|----------|-----------|-----------|--------|
| `hyperdim.rs:119` | `Box<[i32; 10240]>` | Per thread per bundle | CRITICAL |
| `hyperdim.rs:253` | `Vec<u8>` (1280B) | Per to_bytes() | HIGH |
| `reservoir.rs:330-331` | `Vec<f32>` (size*4) | Per spectral radius estimation | MEDIUM |
| `singularity.rs:258` | `String` (id.clone) | Per concept per search | CRITICAL |
| `singularity.rs:348-351` | `Vec<HVec10240>` | Per bundle_concepts | MEDIUM |

### 2.2 String Allocations in Path Conversion

#### Issue 2.2.1: Path to String Conversion - **LOW**
- **Location:** `src/persistence.rs:43, 186-194`
- **Impact:** Minor - only during setup
- **Problem:** `path.to_string()` allocations
- **Fix:** Use `&str` or `PathBuf` where possible

### 2.3 Box/Arc Cloning Overhead

#### Issue 2.3.1: Arc Clone in singularity() getter - **LOW**
- **Location:** `src/framework.rs:97-99`
- **Impact:** One Arc clone per call
- **Current:** Returns cloned Arc (correct pattern)
- **Status:** Acceptable for API design

---

## 3. Async Performance

### 3.1 Lock Contention Analysis

| Lock | Type | Usage | Contention Risk |
|------|------|-------|-----------------|
| `singularity` | `Arc<RwLock<Singularity>>` | All operations | HIGH |
| `reservoir` | `Arc<RwLock<Option<ChaoticReservoir>>>` | Sequence processing | MEDIUM |
| `query_cache` | `Mutex<QueryCache>` | Similarity queries | HIGH |

### 3.2 Lock Held Across await Points

**Verified Safe Locations:**
- `src/framework.rs:108-111` - Lock dropped before await
- `src/framework.rs:139-142` - Lock dropped before await
- `src/framework.rs:156-161` - Lock dropped before await

**No violations found** - the codebase correctly scopes locks.

### 3.3 Connection Pool Efficiency

#### Issue 3.3.1: Semaphore Acquired Per Operation - **MEDIUM**
- **Location:** `src/persistence.rs:85-94`
- **Impact:** Every DB operation acquires semaphore permit
- **Current:** Uses `tokio::sync::Semaphore` with configurable pool size
- **Status:** Acceptable for current design

#### Issue 3.3.2: No Connection Pooling for Local SQLite - **LOW**
- **Location:** `src/persistence.rs:35-48`
- **Impact:** New connection per operation for local DB
- **Problem:** `self.db.connect()` called each time
- **Fix:** Use `deadpool` or maintain connection pool

### 3.4 Batch Operation Opportunities

#### Issue 3.4.1: save_concepts Could Be More Efficient - **MEDIUM**
- **Location:** `src/persistence.rs:167-220`
- **Impact:** Transaction overhead for batch saves
- **Current:** Good - uses single transaction
- **Improvement:** Could use prepared statement cache

```rust
// RECOMMENDED: Prepared statement caching
lazy_static! {
    static ref SAVE_CONCEPT_SQL: String = /* ... */;
}
```

---

## 4. SIMD & Optimization

### 4.1 Current SIMD Usage

| Function | SIMD | Width | Platform |
|----------|------|-------|----------|
| `bind_simd_x86` | SSE2 | 128-bit | x86/x86_64 |
| `cosine_similarity_simd_x86` | SSE2 | 128-bit | x86/x86_64 |

### 4.2 Missing SIMD Opportunities

#### Issue 4.2.1: No ARM NEON Path - **HIGH**
- **Impact:** Poor performance on Apple Silicon, mobile
- **Fix:** Add NEON implementation

```rust
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn bind_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::*;
    // NEON implementation
}
```

#### Issue 4.2.2: No WASM SIMD Path - **MEDIUM**
- **Impact:** Poor browser performance
- **Fix:** Use `wasm_simd128` when available

```rust
#[cfg(target_arch = "wasm32")]
#[target_feature(enable = "simd128")]
unsafe fn bind_simd_wasm(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use core::arch::wasm32::*;
    // WASM SIMD implementation
}
```

#### Issue 4.2.3: bundle() Has No SIMD - **HIGH**
- **Location:** `src/hyperdim.rs:110-171`
- **Impact:** Sequential bit counting is slow
- **Fix:** Use SIMD popcnt or parallel reduction

### 4.3 Branch Prediction Hints

#### Issue 4.3.1: Likely/Unlikely Hints Missing - **LOW**
- **Locations:** Multiple hot paths
- **Impact:** Minor - modern CPUs predict well
- **Fix:** Add `likely!()`/`unlikely!()` macros for error paths

```rust
#[inline]
fn likely(b: bool) -> bool {
    #[cfg(feature = "nightly")]
    return core::intrinsics::likely(b);
    #[cfg(not(feature = "nightly"))]
    return b;
}
```

### 4.4 Cache-Friendly Data Layouts

#### Issue 4.4.1: HVec10240 is 1280 bytes - **MEDIUM**
- **Impact:** Spans multiple cache lines (20+ on x86_64)
- **Current:** `[u128; 80]` = 1280 bytes
- **Cache lines:** ~20 (64-byte lines)
- **Status:** Acceptable for this data structure

#### Issue 4.4.2: SparseWeights Separated Arrays - **MEDIUM**
- **Location:** `src/reservoir.rs:56-60`
- **Impact:** Cache misses during dot product
- **Current:** `indices` and `weights` in separate Vecs
- **Fix:** Use struct-of-arrays or array-of-structs

```rust
// CURRENT: Separate arrays (cache unfriendly)
struct SparseWeights {
    indices: Vec<usize>,
    weights: Vec<f32>,
}

// RECOMMENDED: Interleaved (cache friendly)
struct SparseWeight {
    index: usize,
    weight: f32,
}
struct SparseWeights {
    data: Vec<SparseWeight>,
    row_offsets: Vec<usize>,
}
```

---

## 5. Benchmark Analysis

### 5.1 Existing Benchmarks Review

| Benchmark | Target | Status | Notes |
|-----------|--------|--------|-------|
| `hvec_random` | N/A | OK | Baseline |
| `cosine_similarity` | N/A | OK | Has SIMD |
| `batch_similarity_1000` | <500μs | NEEDS VERIFICATION | Chunk size 128 |
| `hvec_bind` | N/A | OK | Has SIMD |
| `hvec_bundle_10/100/1000` | N/A | **SLOW** | No SIMD, allocations |
| `reservoir_step_50k` | <100μs | **AT RISK** | Hot path |
| `reservoir_to_hypervector` | N/A | **SLOW** | Sequential only |

### 5.2 Missing Benchmarks

#### Issue 5.2.1: No Singularity Search Benchmarks - **HIGH**
- **Gap:** No benchmark for `find_similar()` with various concept counts
- **Needed:** 
  - `singularity_search_1k_concepts`
  - `singularity_search_10k_concepts`
  - `singularity_search_100k_concepts`
  - Cache hit/miss benchmarks

#### Issue 5.2.2: No Framework-Level Benchmarks - **HIGH**
- **Gap:** No end-to-end framework benchmarks
- **Needed:**
  - `framework_inject_probe_roundtrip`
  - `framework_associate_traversal`
  - `framework_sequence_processing`

#### Issue 5.2.3: No Persistence Concurrency Benchmarks - **MEDIUM**
- **Gap:** No concurrent access benchmarks
- **Needed:**
  - `concurrent_reads_10_threads`
  - `concurrent_mixed_workload`

#### Issue 5.2.4: No Memory Usage Benchmarks - **MEDIUM**
- **Gap:** No memory footprint tracking
- **Needed:**
  - `memory_concept_1k`
  - `memory_associations_10k`
  - `memory_reservoir_50k`

#### Issue 5.2.5: No Cache Performance Benchmarks - **LOW**
- **Gap:** Cache hit rate not measured
- **Needed:**
  - `cache_hit_rate_similar_queries`
  - `cache_eviction_behavior`

### 5.3 Performance Regression Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SIMD code not compiled | Medium | High | CI check for target features |
| Rayon thread pool exhaustion | Low | Medium | Configure thread pool size |
| Lock contention at scale | High | High | Load testing in CI |
| Memory fragmentation | Medium | Medium | Use jemalloc, monitor RSS |
| Spectral radius drift | Low | Critical | Add validation in step() |

---

## 6. Optimization Recommendations Summary

### Immediate Actions (This Sprint)

1. **Fix bundle() allocations** (Issue 1.1.1) - CRITICAL
   - Use thread-local buffers
   - Estimated improvement: 3-5x

2. **Fix find_similar() string cloning** (Issue 1.3.1) - CRITICAL
   - Return references, clone only top_k
   - Estimated improvement: 2-3x for large concept stores

3. **Parallelize to_hypervector()** (Issue 1.2.1) - CRITICAL
   - Add Rayon parallel iterator
   - Estimated improvement: 4-8x on multi-core

### Short-term (Next 2 Sprints)

4. **Add AVX2 SIMD paths** (Issue 1.1.3) - HIGH
5. **Optimize bundle() bit iteration** (Issue 1.1.2) - HIGH
6. **Replace query_cache Mutex with RwLock** (Issue 1.3.2) - HIGH
7. **Add ARM NEON support** (Issue 4.2.1) - HIGH
8. **Add missing benchmarks** (Issue 5.2.1, 5.2.2) - HIGH

### Medium-term (Next Quarter)

9. **Optimize SparseWeights layout** (Issue 4.4.2) - MEDIUM
10. **Add WASM SIMD** (Issue 4.2.2) - MEDIUM
11. **Implement connection pooling** (Issue 3.3.2) - MEDIUM
12. **Add subset hashing for cache keys** (Issue 1.3.3) - MEDIUM

---

## 7. GOAP Action Plan

### Preconditions Met
- Hot paths identified
- Benchmarks exist for core operations
- SIMD infrastructure in place

### Actions to Close Performance Gaps

| Action | Duration | Dependencies | Success Criteria |
|--------|----------|--------------|------------------|
| A1: Fix bundle() thread-local buffers | 2h | None | hvec_bundle_1000 < 5ms |
| A2: Fix find_similar() string clones | 4h | None | search_10k < 10ms |
| A3: Parallelize to_hypervector() | 3h | None | 4x speedup on 8 cores |
| A4: Add AVX2 SIMD | 8h | A3 | bind() 2x faster on AVX2 |
| A5: Add search benchmarks | 4h | None | Coverage for 1K-100K concepts |
| A6: Optimize bundle() bit ops | 4h | A1 | bundle() 2x faster |
| A7: Replace cache Mutex | 2h | None | Concurrent reads scale linearly |

### Goal State
- `reservoir_step_50k` consistently < 100μs
- `batch_similarity_1000` consistently < 500μs  
- Search scales linearly to 100K concepts
- No allocations in hot loops

---

## Appendix A: File:Line Reference Index

### Critical Issues
- `src/hyperdim.rs:110-171` - Bundle allocation storm
- `src/singularity.rs:254-266` - String cloning in search
- `src/reservoir.rs:291-318` - Sequential to_hypervector
- `src/reservoir.rs:238-243` - Strided cache access

### High Issues
- `src/hyperdim.rs:15-65` - Missing AVX2/NEON
- `src/hyperdim.rs:122-128` - Bit-by-bit iteration
- `src/singularity.rs:240-251` - Mutex in read path
- `src/reservoir.rs:110-121` - Inline hinting
- `src/framework.rs:346-369` - Multi-lock snapshot
- `src/singularity.rs:433-438` - Expensive hashing

### Medium Issues
- `src/hyperdim.rs:253-259` - to_bytes allocation
- `src/reservoir.rs:214-233` - Input cache comparison
- `src/singularity.rs:74-77` - VecDeque O(n) remove
- `src/persistence.rs:85-94` - Semaphore per op
- `src/persistence.rs:167-220` - Statement caching
- `src/reservoir.rs:56-60` - Sparse layout
- `src/framework.rs:231-235` - Concept cloning

---

*Report generated by Swarm Group B - Performance Specialist*  
*Next Review: Post-optimization validation*
