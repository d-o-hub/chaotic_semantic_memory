# [ADR-0024] Performance Optimizations Phase 2

## Status
Deferred (Post-1.0)

**Rationale**: Analysis Swarm Consensus (2026-02-17) determined all Phase 1 performance targets are met (reservoir_step <100μs). Phase 2 optimizations (SIMD completion, Product Quantization, LSH) provide diminishing returns and add significant complexity. Reconsider when:
- User count exceeds 200k concepts with latency degradation
- 10M concept memory budget requires quantization
- Specific user requests for sub-linear search

## Context and Problem Statement

While Phase 1 performance targets are met (reservoir step <100μs), analysis reveals remaining optimization opportunities:

1. **Hypervector Operations**: `hamming_distance`, `permute`, and `bundle` bit-counting lack SIMD acceleration
2. **Reservoir Conversion**: `to_hypervector()` is sequential and unoptimized
3. **Scale Challenge**: 10M concept target requires sub-linear search; current O(n) scan will not suffice
4. **Missing Benchmarks**: Several hot paths are not benchmarked, preventing regression detection

## Decision Drivers

- Must maintain reservoir_step <100μs target
- WASM compatibility must be preserved (all SIMD gated)
- No hardcoded magic numbers - use configurable constants
- Memory budget: 12MB for 10M concept compressed index
- Query latency target: <10ms p99 for similarity search

## Considered Options

### Option 1: Incremental Optimizations
Add SIMD to remaining hypervector operations, parallelize `to_hypervector()`, add missing benchmarks.

**Pros:**
- Low risk, incremental improvements
- Maintains exact search semantics
- No architectural changes

**Cons:**
- O(n) scan remains for similarity search
- Does not address 10M concept scale challenge

### Option 2: Product Quantization + SIMD
Implement product quantization for 16x compression + fast approximate search, with SIMD-accelerated distance computation.

**Pros:**
- Fits 10M concepts in 12MB budget
- 10x+ query speedup via lookup tables
- Still allows exact re-ranking for top-k

**Cons:**
- Approximate results (configurable accuracy)
- Added complexity for quantization training
- New API surface for quantized index

### Option 3: Locality Sensitive Hashing
Implement multi-probe LSH index for sub-linear approximate search.

**Pros:**
- Natural fit for binary hypervectors
- No training required (random projections)
- Sub-linear query time

**Cons:**
- Memory overhead (~2x for hash tables)
- Tuning required for hash table count/size
- Approximate only (no exact re-rank trivial)

### Option 4: Combined Approach (Recommended)
Layer optimizations: SIMD improvements → Product Quantization → LSH fallback

**Pros:**
- Best of all approaches
- Progressive enhancement
- Can disable features via config

**Cons:**
- More complex implementation
- Requires careful integration testing

## Decision Outcome

Chosen option: **Option 4 - Combined Approach with Progressive Enhancement**

### Implementation Phases

#### Phase 2A: SIMD Completion (Immediate)
- Add SIMD path for `hamming_distance()` following ADR-0013 pattern
- Parallelize `to_hypervector()` with Rayon
- Add all missing benchmarks
- **Target:** 2-3x speedup for affected operations

#### Phase 2B: Product Quantization (Medium)
- Implement `QuantizedIndex` structure
- K-means clustering for subspace codebooks
- Asymmetric distance computation
- **Target:** 16x memory reduction, 10x query speedup

#### Phase 2C: LSH Index (Medium)
- Implement `LshIndex` with configurable tables/bits
- Multi-probe query strategy
- Integration with Singularity for large concept counts
- **Trigger:** When concept count > 200k and brute-force latency > 10ms

### Configuration Constants

```rust
// src/hyperdim.rs
pub const PQ_SUBSPACES: usize = 80;        // 10240 / 128 bits
pub const PQ_CLUSTERS: usize = 256;        // 1 byte per subspace
pub const LSH_DEFAULT_TABLES: usize = 20;
pub const LSH_DEFAULT_BITS: usize = 16;
pub const LSH_TRIGGER_THRESHOLD: usize = 200_000;
```

### Positive Consequences

- Achieves 10M concept target within memory budget
- Sub-10ms query latency for large concept stores
- Maintains exact semantics for small stores (automatic fallback)
- No WASM compatibility issues (all SIMD gated)

### Negative Consequences

- Increased code complexity with quantization layer
- Approximate results for very large stores (documented trade-off)
- Additional memory during quantization training
- New failure modes in K-means clustering

## Implementation Sketch

### SIMD Hamming Distance
```rust
impl HVec10240 {
    #[cfg(all(not(target_arch = "wasm32"), 
              any(target_arch = "x86_64", target_arch = "x86")))]
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        use std::arch::x86_64::*;
        unsafe {
            let mut acc = _mm256_setzero_si256();
            for i in (0..80).step_by(2) {
                let a = _mm256_loadu_si256(&self.data[i] as *const _ as *const __m256i);
                let b = _mm256_loadu_si256(&other.data[i] as *const _ as *const __m256i);
                let xor = _mm256_xor_si256(a, b);
                // Popcnt via lookup or _mm_popcnt_u64 on halves
                acc = _mm256_add_epi64(acc, popcnt_256(xor));
            }
            // Sum lanes...
        }
    }
}
```

### Product Quantization
```rust
pub struct QuantizedIndex {
    subspace_centroids: Vec<[u128; 256]>,  // 80 subspaces × 256 centroids
    codes: Vec<[u8; 80]>,                   // Per-concept codes
    concept_ids: Vec<String>,
}

impl QuantizedIndex {
    pub fn build(concepts: &[(String, HVec10240)]) -> Result<Self> {
        // K-means per subspace
        // Assign codes
        // Return index
    }
    
    pub fn search(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
        // Precompute query-to-centroid distances
        // Scan codes, sum lookups
        // Re-rank top candidates with exact similarity
    }
}
```

### LSH Index
```rust
pub struct LshIndex {
    tables: Vec<HashMap<u64, Vec<usize>>>,  // table -> hash -> concept indices
    hash_funcs: Vec<Vec<usize>>,         // Random bit positions per table
}

impl LshIndex {
    fn hash(&self, vector: &HVec10240, table: usize) -> u64 {
        // Sample bits at hash_funcs[table] positions
        // Pack into u64 hash
    }
    
    pub fn query(&self, vector: &HVec10240, top_k: usize) -> Vec<usize> {
        // Multi-probe: get candidates from multiple tables
        // Deduplicate
        // Return candidate indices for re-ranking
    }
}
```

## Links

- ADR-0013: SIMD-Accelerated Hypervector Operations
- ADR-0007: Similarity Search Optimization
- ADR-0004: Sparse Reservoir Weight Matrix
- Analysis: `plans/handoffs/analysis_group_b_performance.md`
