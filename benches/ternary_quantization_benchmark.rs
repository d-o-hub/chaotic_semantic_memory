//! Benchmarks for ternary quantization (ADR-0076) performance.
//!
//! Compares ternary scalar product, cosine similarity (on ternary), and
//! baseline hamming distance across input dimensions.
//! Also benchmarks LSH query modification (query center estimation).

// Casts are intentional for benchmark metrics
// Clones needed for benchmark measurement isolation
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::redundant_clone
)]

use chaotic_semantic_memory::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use csm_core::hyperdim_ternary::TernaryHVec;

/// Input f32 lengths to benchmark quantization at.
const DIMENSIONS: [usize; 3] = [1024, 4096, 10240];

/// Estimate a query center by bundling candidate hypervectors.
///
/// Simulates LSH query modification: given N candidate vectors from
/// hash buckets, bundle them into a single representative center vector.
fn estimate_query_center(candidates: &[HVec10240]) -> HVec10240 {
    HVec10240::bundle(candidates).unwrap()
}

// ─── Ternary vs Hamming comparison ──────────────────────────────────────────

fn bench_ternary_vs_hamming(c: &mut Criterion) {
    let mut group = c.benchmark_group("ternary_vs_hamming");

    // Pre-generate f32 data at each dimension for TernaryHVec creation.
    let f32_data: Vec<Vec<f32>> = DIMENSIONS
        .iter()
        .map(|&dim| (0..dim).map(|i| (i as f32 * 0.1).sin() * 0.5).collect())
        .collect();

    // Pre-create TernaryHVec pairs at each dimension.
    let ternary_pairs: Vec<(TernaryHVec, TernaryHVec)> = f32_data
        .iter()
        .map(|data| {
            let a = TernaryHVec::from_f32_slice(data);
            // Offset the second vector for a realistic non-trivial comparison.
            let b_data: Vec<f32> = data.iter().map(|v| v + 0.2).collect();
            let b = TernaryHVec::from_f32_slice(&b_data);
            (a, b)
        })
        .collect();

    // Fixed HVec10240 pair for hamming baseline (always 10240 bits).
    let h_a = HVec10240::random();
    let h_b = HVec10240::random();

    for (idx, &dim) in DIMENSIONS.iter().enumerate() {
        let (t_a, t_b) = &ternary_pairs[idx];

        // Baseline: HVec10240 hamming distance (SIMD-accelerated).
        group.bench_with_input(BenchmarkId::new("hamming_distance", dim), &dim, |b, _| {
            b.iter(|| black_box(h_a).hamming_distance(black_box(&h_b)))
        });

        // Ternary scalar product: bitwise AND/XOR + popcount.
        group.bench_with_input(
            BenchmarkId::new("ternary_scalar_product", dim),
            &dim,
            |b, _| b.iter(|| black_box(t_a).ternary_scalar_product(black_box(t_b))),
        );

        // Ternary cosine similarity: scalar product / DIMENSION.
        group.bench_with_input(
            BenchmarkId::new("ternary_cosine_similarity", dim),
            &dim,
            |b, _| b.iter(|| black_box(t_a).cosine_similarity(black_box(t_b))),
        );
    }

    group.finish();
}

// ─── LSH query modification ────────────────────────────────────────────────

fn bench_lsh_query_modification(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsh_query_modification");

    for &candidate_count in &[50, 200] {
        let candidates: Vec<HVec10240> =
            (0..candidate_count).map(|_| HVec10240::random()).collect();

        group.bench_with_input(
            BenchmarkId::new("estimate_query_center", candidate_count),
            &candidates,
            |b, cands| b.iter(|| black_box(estimate_query_center(black_box(cands)))),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_ternary_vs_hamming,
    bench_lsh_query_modification,
);
criterion_main!(benches);
