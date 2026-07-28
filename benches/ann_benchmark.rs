#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::redundant_clone
)]
//! ANN scale benchmarks: compare BruteForce, HNSW, and LSH at 1k/10k/50k concept scales.
//! HNSW and LSH arms are gated behind `ann-hnsw` / `ann-lsh` feature flags.

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::singularity::{ConceptBuilder, Singularity, SingularityConfig};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

const NS: &str = "_default";
const ANN_BENCH_TOP_K: usize = 10;
const ANN_BENCH_SAMPLE_SIZE: usize = 10;
const ANN_BENCH_WARMUP_SECS: u64 = 1;
const ANN_BENCH_MEASUREMENT_SECS: u64 = 5;
const ANN_BENCH_SCALES: [usize; 3] = [1_000, 10_000, 50_000];

/// Build a `Singularity` pre-loaded with `count` seeded vectors using the given backend.
/// Setup cost is outside the measurement loop.
fn build_ann_singularity(count: usize, backend: IndexBackend) -> Singularity {
    let config = SingularityConfig {
        max_cached_top_k: 0,
        index_backend: backend,
        ..Default::default()
    };
    let mut sing = Singularity::with_config(config);
    for i in 0..count {
        sing.inject(
            NS,
            ConceptBuilder::new(format!("v{i}"))
                .with_vector(HVec10240::new_seeded(i as u64))
                .build()
                .unwrap(),
        )
        .unwrap();
    }
    sing
}

fn bench_ann_brute_force(c: &mut Criterion) {
    let mut group = c.benchmark_group("ann_brute_force");
    group.sample_size(ANN_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(ANN_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(ANN_BENCH_MEASUREMENT_SECS));

    let query = HVec10240::new_seeded(9999);
    for &n in &ANN_BENCH_SCALES {
        let sing = build_ann_singularity(n, IndexBackend::BruteForce);
        group.bench_function(format!("search_{n}"), |b| {
            b.iter(|| black_box(sing.find_similar_cached(NS, black_box(&query), ANN_BENCH_TOP_K)))
        });
    }
    group.finish();
}

#[cfg(feature = "ann-hnsw")]
fn bench_ann_hnsw(c: &mut Criterion) {
    let mut group = c.benchmark_group("ann_hnsw");
    group.sample_size(ANN_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(ANN_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(ANN_BENCH_MEASUREMENT_SECS));

    let query = HVec10240::new_seeded(9999);
    let backend = IndexBackend::Hnsw {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
    };
    for &n in &ANN_BENCH_SCALES {
        let sing = build_ann_singularity(n, backend.clone());
        group.bench_function(format!("search_{n}"), |b| {
            b.iter(|| black_box(sing.find_similar_cached(NS, black_box(&query), ANN_BENCH_TOP_K)))
        });
    }
    group.finish();
}

#[cfg(not(feature = "ann-hnsw"))]
#[allow(clippy::missing_const_for_fn)]
fn bench_ann_hnsw(_c: &mut Criterion) {}

#[cfg(feature = "ann-lsh")]
fn bench_ann_lsh(c: &mut Criterion) {
    let mut group = c.benchmark_group("ann_lsh");
    group.sample_size(ANN_BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(ANN_BENCH_WARMUP_SECS));
    group.measurement_time(Duration::from_secs(ANN_BENCH_MEASUREMENT_SECS));

    let query = HVec10240::new_seeded(9999);
    let backend = IndexBackend::Lsh {
        num_tables: 8,
        hash_bits: 16,
    };
    for &n in &ANN_BENCH_SCALES {
        let sing = build_ann_singularity(n, backend.clone());
        group.bench_function(format!("search_{n}"), |b| {
            b.iter(|| black_box(sing.find_similar_cached(NS, black_box(&query), ANN_BENCH_TOP_K)))
        });
    }
    group.finish();
}

#[cfg(not(feature = "ann-lsh"))]
#[allow(clippy::missing_const_for_fn)]
fn bench_ann_lsh(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_ann_brute_force,
    bench_ann_hnsw,
    bench_ann_lsh
);
criterion_main!(benches);
