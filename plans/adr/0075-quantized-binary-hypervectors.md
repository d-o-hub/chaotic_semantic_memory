# ADR-0075: Quantized Binary Hypervectors

## Status

Proposed (2026-04-30) — defers active work; opt-in when warranted

## Context and Problem Statement

Current hypervector storage:
- 10240 dims × 4 bytes (f32) = **40 KB per concept**
- 1M concepts = 40 GB RAM
- 25M concepts = 1 TB RAM (practical ceiling on commodity hardware)

Binary hypervectors (Kanerva-style):
- 10240 bits packed into 160 × u64 = **1.28 KB per concept** (32× compression)
- Distance is Hamming via popcount — already SIMD-accelerated on AVX2/NEON
- 1M concepts = 1.28 GB
- 25M concepts = 32 GB
- 800M concepts = 1 TB (32× scale increase)

Tradeoffs:
- Binary HVs lose ~5% retrieval accuracy vs f32 (well-established in HDC literature)
- Bind/bundle operations must use majority rule (XOR/popcount) instead of element-wise multiply

Current PR #129 already uses integer Hamming for ranking — this ADR completes the picture by also storing as binary.

## Decision Drivers

- Opt-in — small deployments stay f32 (better accuracy)
- Backward compatible (per-database flag)
- Bind/bundle/projection algebra still works
- WASM compatible
- Must not regress current f32 performance

## Considered Options

1. **Add `BinaryHypervector` type alongside `HVec10240`** with feature flag and per-DB choice
2. Replace HVec10240 entirely with binary
3. Keep f32, add binary as a separate index only (already done partially in PR #129)

## Decision Outcome

Chosen: **Option 1** — add as alternative, opt-in. Provides scale path without disturbing accuracy-sensitive users.

## Implementation

### New types

```rust
// src/hyperdim_binary.rs (≤ 400 LOC)
pub struct BHVec10240 {
    bits: [u64; 160],   // 10240 bits packed
}

impl BHVec10240 {
    pub fn from_f32(v: &HVec10240) -> Self;       // sign quantization
    pub fn to_f32(&self) -> HVec10240;            // sign expansion
    pub fn xor(&self, other: &Self) -> Self;
    pub fn hamming(&self, other: &Self) -> u32;   // popcount
    pub fn bundle(vecs: &[&Self]) -> Self;        // majority rule
    pub fn permute(&self, n: u32) -> Self;        // cyclic shift
}
```

### Trait abstraction

```rust
// src/hyperdim/mod.rs
pub trait Hypervector: Sized {
    type Distance: PartialOrd + Copy;
    fn distance(&self, other: &Self) -> Self::Distance;
    fn bind(&self, other: &Self) -> Self;
    fn bundle(vecs: &[&Self]) -> Self;
}

impl Hypervector for HVec10240 { /* ... */ }
impl Hypervector for BHVec10240 { /* ... */ }
```

### Singularity generic over Hypervector

```rust
pub struct Singularity<H: Hypervector = HVec10240> {
    concepts: HashMap<String, Concept<H>>,
    ...
}

pub type BinarySingularity = Singularity<BHVec10240>;
```

### Persistence

- Schema: add `vector_format` column (`f32` | `binary`)
- Migration `007_add_vector_format.sql`
- Existing rows tagged `f32` (backward compatible)
- Binary rows store packed bytes (BLOB, 1.28 KB)

### Cargo feature

```toml
[features]
hv-binary = []   # pure Rust, no extra deps
```

### Benchmarks

- `bench_binary_distance_50k_vs_f32_50k` — should be ≥ 4× faster
- `bench_binary_storage_1m` — should fit in <1.5 GB
- `bench_binary_recall@10_vs_f32` — accuracy delta report

## Pros and Cons

### Pros
- 32× memory compression
- Faster distance (popcount vs f32 dot product)
- Removes the practical scale ceiling

### Cons
- ~5% accuracy loss (workload-dependent)
- Type churn — Singularity becomes generic
- Migration complexity for users who switch formats mid-life
- Bind/bundle semantics shift slightly (sign-only)

## Acceptance Criteria

- [ ] `BHVec10240` type implemented with full algebra
- [ ] `Hypervector` trait + impls for both types
- [ ] `Singularity<H>` generic
- [ ] `binary` opt-in via FrameworkBuilder
- [ ] Persistence migration `007_add_vector_format.sql`
- [ ] Recall@10 vs f32 benchmark report
- [ ] Memory benchmark proves 32× compression
- [ ] All HV files ≤ 400 LOC
