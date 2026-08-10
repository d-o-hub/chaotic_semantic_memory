#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chaotic_semantic_memory::BHVec10240;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

fn bench_bhvec_bundle(c: &mut Criterion) {
    let mut group = c.benchmark_group("bhvec_bundle");
    for n in [2usize, 10, 100, 1000] {
        let vectors: Vec<BHVec10240> = (0..n).map(|_| BHVec10240::random()).collect();
        let vector_refs: Vec<&BHVec10240> = vectors.iter().collect();
        group.bench_with_input(BenchmarkId::from_parameter(n), &vector_refs, |b, refs| {
            b.iter(|| BHVec10240::bundle(black_box(refs)));
        });
    }
    group.finish();
}

fn bench_bhvec_hamming(c: &mut Criterion) {
    let v1 = BHVec10240::random();
    let v2 = BHVec10240::random();

    c.bench_function("bhvec_hamming", |b| {
        b.iter(|| black_box(&v1).hamming(black_box(&v2)));
    });
}

fn bench_bhvec_permute(c: &mut Criterion) {
    let v = BHVec10240::random();
    c.bench_function("bhvec_permute", |b| {
        b.iter(|| black_box(&v).permute(black_box(321)));
    });
}

fn bench_bhvec_serialization(c: &mut Criterion) {
    let v = BHVec10240::random();
    let bytes = v.to_bytes();

    c.bench_function("bhvec_to_bytes", |b| b.iter(|| black_box(&v).to_bytes()));
    c.bench_function("bhvec_from_bytes", |b| {
        b.iter(|| BHVec10240::from_bytes(black_box(&bytes)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_bhvec_bundle,
    bench_bhvec_hamming,
    bench_bhvec_permute,
    bench_bhvec_serialization
);
criterion_main!(benches);
