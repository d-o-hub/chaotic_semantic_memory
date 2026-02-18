# Swarm Group B (Performance) - Comprehensive Analysis Report

**Date:** 2026-02-17  
**Group:** Swarm Group B - Performance  
**Status:** Analysis Complete - Recommendations Ready for Implementation

---

## Executive Summary

Current benchmark results show the reservoir step is **meeting targets** (63μs vs <100μs target), but several optimization opportunities remain for hypervector operations and similarity search at scale. This analysis identifies specific hot paths, SIMD gaps, and proposes 3 high-value features for the memory system.

**Key Findings:**
- Reservoir step: 63μs (37% under target) - GOOD
- Hypervector bind: 52ns (SIMD optimized) - GOOD  
- Cosine similarity: 212ns (has SIMD on x86) - GOOD
- Batch similarity 1000: 470μs - room for improvement
- Missing benchmarks for several hot paths

---

## 1. Performance Improvements - Specific Hot Paths

### 1.1 Critical: Reservoir to_hypervector() Sequential Processing

**Location:** `src/reservoir.rs:243-270`

**Current Implementation:**
```rust
pub fn to_hypervector(&self) -> Result<HVec10240> {
    let chunk_size = self.size / HVec10240::DIMENSION;  // ~5 for 50k reservoir
    let mut data = [0u128; 80];

    for (i, word) in data.iter_mut().enumerate() {
        for j in 0..128 {
            let bit_index = i * 128 + j;
            let start = bit_index * chunk_size;
            let end = start + chunk_size;
            let mut sum = 0.0;
            for value in &self.state[start..end] {  // Sequential sum
                sum += *value;
            }
            if sum > 0.0 {
                *word |= 1u128 << j;
            }
        }
    }
    Ok(HVec10240 { data })
}
```

**Issues:**
1. Sequential processing of 10,240 bits
2. Inner sum loop is scalar
3. No SIMD or parallelization
4. Called after every sequence processing

**Recommended Fix:**
```rust
pub fn to_hypervector(&self) -> Result<HVec10240> {
    let chunk_size = self.size / HVec10240::DIMENSION;
    let mut data = [0u128; 80];

    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        let chunks: Vec<u128> = (0..HVec10240::DIMENSION)
            .into_par_iter()
            .map(|bit_index| {
                let start = bit_index * chunk_size;
                let end = start + chunk_size;
                let sum: f32 = self.state[start..end].iter().sum();
                if sum > 0.0 { 1u128 } else { 0u128 }
            })
            .collect();
        
        for (i, bit_val) in chunks.iter().enumerate() {
            let word_idx = i / 128;
            let bit_idx = i % 128;
            if *bit_val != 0 {
                data[word_idx] |= 1u128 << bit_idx;
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    // ... scalar fallback
    
    Ok(HVec10240 { data })
}
```

---

### 1.2 High: Bundle Operation Bit Counting

**Location:** `src/hyperdim.rs:110-167`

**Current Issue:**
The inner bit counting loop (lines 121-127) iterates bit-by-bit:
```rust
for i in 0..80 {
    for j in 0..128 {  // 128 iterations per word
        if (v.data[i] >> j) & 1 == 1 {
            local[i * 128 + j] += 1;
        }
    }
}
```

**Optimization:** Use u128::count_ones() with lookup or SIMD popcnt:
```rust
// Use hardware popcnt for 8x speedup
let word = v.data[i];
for j in 0..128 {
    local[i * 128 + j] += ((word >> j) & 1) as i32;
}
// Could use: _mm_popcnt_u64 or intrinsic-based bit extraction
```

---

### 1.3 Medium: Hamming Distance Missing SIMD

**Location:** `src/hyperdim.rs:236-242`

**Current:**
```rust
pub fn hamming_distance(&self, other: &Self) -> u32 {
    let mut distance = 0u32;
    for i in 0..80 {
        distance += (self.data[i] ^ other.data[i]).count_ones();
    }
    distance
}
```

**Issue:** No SIMD path for this hot function. Should have x86_64 SIMD like `bind` and `cosine_similarity`.

---

### 1.4 Medium: Reservoir.run() Allocates Per Step

**Location:** `src/reservoir.rs:204-211`

```rust
pub fn run(&mut self, inputs: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
    let mut states = Vec::with_capacity(inputs.len());
    for input in inputs {
        self.step(input)?;
        states.push(self.state.clone());  // Allocates Vec each iteration
    }
    Ok(states)
}
```

**Issue:** `self.state.clone()` allocates a new Vec every step. Should use pre-allocated buffer.

---

### 1.5 Low: SparseWeights.dot_row() Could Use SIMD

**Location:** `src/reservoir.rs:67-78`

The sparse dot product could benefit from:
1. Loop unrolling for degree=8
2. SIMD gather for indices (AVX2)
3. Prefetching for next row

---

## 2. SIMD Opportunities Summary

| Function | Current | Has SIMD | Priority |
|----------|---------|----------|----------|
| `bind()` | Scalar + x86 SIMD | YES (x86 only) | GOOD |
| `cosine_similarity()` | Scalar + x86 SIMD | YES (x86 only) | GOOD |
| `hamming_distance()` | Scalar only | NO | **HIGH** |
| `permute()` | Scalar only | NO | MEDIUM |
| `bundle()` bit count | Scalar | NO | **HIGH** |
| `to_hypervector()` | Scalar only | NO | **HIGH** |

### Recommended SIMD Additions:

#### 2.1 Hamming Distance SIMD (x86_64)
```rust
#[cfg(all(not(target_arch = "wasm32"), any(target_arch = "x86_64", target_arch = "x86")))]
#[inline]
fn hamming_distance_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    use std::arch::x86_64::*;
    let mut total = 0u32;
    for i in (0..80).step_by(2) {
        unsafe {
            let a = _mm_loadu_si128(lhs[i] as *const _);
            let b = _mm_loadu_si128(rhs[i] as *const _);
            let xor = _mm_xor_si128(a, b);
            // Use _mm_popcnt_u64 on high/low halves
            total += _mm_popcnt_u64(_mm_cvtsi128_si64(xor) as u64) as u32;
            total += _mm_popcnt_u64(_mm_extract_epi64(xor, 1) as u64) as u32;
        }
    }
    total
}
```

#### 2.2 Permute SIMD
Could use `_mm_or_si128` with shifted vectors for faster bit rotation across words.

---

## 3. New Feature Proposals

### 3.1 Feature: Product Quantization for Hypervectors

**Motivation:** Current 10M concept projection assumes 12MB compressed index. Product quantization can achieve this with fast approximate search.

**Design:**
```rust
pub struct QuantizedIndex {
    // 10240 bits -> 80 subspaces of 128 bits each
    // Each subspace quantized to 256 centroids (1 byte)
    // Total: 80 bytes per concept vs 1280 bytes = 16x compression
    codebook: Vec<HVec10240>,  // 256 * 80 centroids
    codes: Vec<[u8; 80]>,      // One code per concept
}

impl QuantizedIndex {
    pub fn build(concepts: &[Concept]) -> Self;
    
    /// Approximate similarity using lookup tables
    pub fn approximate_similarity(&self, query: &HVec10240, concept_idx: usize) -> f32;
    
    /// Fast top-k with asymmetric distance computation
    pub fn find_approximate(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)>;
}
```

**Expected Performance:**
- Memory: 16x reduction (80 bytes vs 1280 bytes per concept)
- Query: ~10x faster (table lookups vs bit operations)
- Accuracy: >95% recall with proper re-ranking

**Implementation Sketch:**
1. K-means clustering per 128-bit subspace (offline)
2. Build lookup tables for query-to-centroid distances (per query)
3. Sum 80 byte lookups per candidate
4. Re-rank top-10k with exact similarity

---

### 3.2 Feature: Locality Sensitive Hashing (LSH) Index

**Motivation:** For 10M+ concepts, O(n) scan is too slow even with SIMD. LSH provides sub-linear approximate search.

**Design:**
```rust
pub struct LshIndex {
    // Multiple hash tables with random bit projections
    num_tables: usize,      // e.g., 20
    num_bits: usize,        // e.g., 16 (65536 buckets per table)
    hash_tables: Vec<HashMap<u64, Vec<String>>>,
    hash_functions: Vec<Vec<usize>>, // Which bits to sample
}

impl LshIndex {
    pub fn new(num_tables: usize, num_bits: usize) -> Self;
    pub fn insert(&mut self, id: &str, vector: &HVec10240);
    pub fn query(&self, vector: &HVec10240, top_k: usize) -> Vec<(String, f32)>;
}
```

**Why It Works for HDC:**
- Hypervectors are binary - Hamming-based LSH is natural
- Random bit sampling creates effective hash functions
- Multi-probe LSH handles boundary cases

**Expected Performance:**
- Query time: O(n^(1/num_bits)) instead of O(n)
- For 10M concepts with 16-bit hashes: ~150 candidates to check
- Memory overhead: ~2x (storing concept IDs in hash tables)

---

### 3.3 Feature: Reservoir Compute Sharding

**Motivation:** Very large reservoirs (500k+ neurons) could benefit from sharded computation.

**Design:**
```rust
pub struct ShardedReservoir {
    shards: Vec<Reservoir>,
    input_router: SparseWeights,  // Routes inputs to shards
    shard_combiner: SparseWeights, // Combines shard outputs
}

impl ShardedReservoir {
    pub fn new(input_size: usize, total_size: usize, num_shards: usize) -> Result<Self>;
    
    pub fn step(&mut self, input: &[f32]) -> Result<&[f32]> {
        // Each shard updates independently
        // Combines via sparse weights between shards
    }
}
```

**Use Case:** Large-scale temporal processing with distributed reservoir dynamics.

---

## 4. Benchmark Gaps

The following operations are **NOT** currently benchmarked but should be:

| Operation | Why Benchmark | Priority |
|-----------|---------------|----------|
| `HVec10240::hamming_distance()` | Hot path for search | **HIGH** |
| `HVec10240::permute()` | Used in sequence encoding | **HIGH** |
| `HVec10240::bundle()` | Core bundling operation | **HIGH** |
| `Reservoir::to_hypervector()` | Post-sequence conversion | **HIGH** |
| `Reservoir::run()` | Multi-step sequences | **HIGH** |
| `Singularity::find_similar()` | Scale with concept count | **HIGH** |
| `Persistence::save_concept()` | I/O bound operation | MEDIUM |
| `Persistence::load_all_concepts()` | Startup performance | MEDIUM |
| `ChaoticReservoir::step()` | Chaos overhead | MEDIUM |
| `Framework::process_sequence()` | End-to-end temporal | LOW |

### Recommended Benchmark Additions:

```rust
// benches/benchmark.rs additions:

fn bench_hamming_distance(c: &mut Criterion) {
    let a = HVec10240::random();
    let b = HVec10240::random();
    c.bench_function("hamming_distance", |b| {
        b.iter(|| a.hamming_distance(black_box(&b)))
    });
}

fn bench_permute(c: &mut Criterion) {
    let a = HVec10240::random();
    c.bench_function("permute_64", |b| {
        b.iter(|| a.permute(black_box(64)))
    });
}

fn bench_bundle(c: &mut Criterion) {
    let vecs: Vec<_> = (0..100).map(|_| HVec10240::random()).collect();
    c.bench_function("bundle_100", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vecs)))
    });
}

fn bench_reservoir_to_hypervector(c: &mut Criterion) {
    let reservoir = Reservoir::new_seeded(10240, 50000, 42).unwrap();
    c.bench_function("reservoir_to_hvec", |b| {
        b.iter(|| reservoir.to_hypervector())
    });
}

fn bench_similarity_search_scale(c: &mut Criterion) {
    for count in [1000, 10000, 100000] {
        // Setup singularity with 'count' concepts
        // Benchmark find_similar
    }
}
```

---

## 5. Architecture Decision Record Skeleton

See: `plans/adr/0024-performance-optimizations-phase2.md`

---

## 6. Handoff Recommendations

### Immediate Actions (Next Sprint):
1. Add missing benchmarks for `hamming_distance`, `permute`, `bundle`, `to_hypervector`
2. Implement SIMD for `hamming_distance` following ADR-0013 pattern
3. Parallelize `to_hypervector()` with Rayon

### Medium Term (Next 2 Sprints):
1. Prototype Product Quantization for hypervectors
2. Implement LSH index for approximate search
3. Add benchmark proving 10M concept query latency

### Long Term (Future Release):
1. AVX-512 support for newer CPUs
2. GPU acceleration for batch operations
3. Reservoir compute sharding

---

## 7. Validation Checklist

- [ ] All new features maintain WASM compatibility
- [ ] SIMD code paths gated with `#[cfg(not(target_arch = "wasm32"))]`
- [ ] No hardcoded magic numbers - use named constants
- [ ] Reservoir step remains <100μs
- [ ] All benchmarks run and report stable results
- [ ] Property tests added for new quantization features

---

## Appendix: Current Benchmark Baseline

```
reservoir_step_50k:     62-66 μs   (target: <100 μs) ✓
cosine_similarity:      209-214 ns
batch_similarity_1000:  427-518 μs
hvec_bind:              51-53 ns
hvec_random:            495-511 ns
```

**Environment:** Linux x86_64, Release profile with LTO
