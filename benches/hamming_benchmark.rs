use chaotic_semantic_memory::HVec10240;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_hamming_distance(c: &mut Criterion) {
    let v1 = HVec10240::random();
    let v2 = HVec10240::random();

    c.bench_function("hamming_distance_single", |b| {
        b.iter(|| black_box(&v1).hamming_distance(black_box(&v2)))
    });
}

fn bench_hamming_distance_batch(c: &mut Criterion) {
    let query = HVec10240::random();
    let candidates: Vec<_> = (0..1000).map(|_| HVec10240::random()).collect();

    c.bench_function("hamming_distance_batch_1000", |b| {
        b.iter(|| {
            for candidate in &candidates {
                black_box(black_box(&query).hamming_distance(black_box(candidate)));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_hamming_distance,
    bench_hamming_distance_batch
);
criterion_main!(benches);
