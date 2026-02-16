use chaotic_semantic_memory::reservoir::Reservoir;
use chaotic_semantic_memory::HVec10240;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_hvec_creation(c: &mut Criterion) {
    c.bench_function("hvec_random", |b| b.iter(HVec10240::random));
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let a = HVec10240::random();
    let other = HVec10240::random();

    c.bench_function("cosine_similarity", |bencher| {
        bencher.iter(|| a.cosine_similarity(black_box(&other)))
    });
}

fn bench_batch_similarity(c: &mut Criterion) {
    let query = HVec10240::random();
    let candidates: Vec<_> = (0..1000).map(|_| HVec10240::random()).collect();

    c.bench_function("batch_similarity_1000", |b| {
        b.iter(|| {
            chaotic_semantic_memory::hyperdim::batch_cosine_similarity(
                black_box(&query),
                black_box(&candidates),
            )
        })
    });
}

fn bench_binding(c: &mut Criterion) {
    let a = HVec10240::random();
    let other = HVec10240::random();

    c.bench_function("hvec_bind", |bencher| {
        bencher.iter(|| a.bind(black_box(&other)))
    });
}

fn bench_reservoir_step_50k(c: &mut Criterion) {
    let mut reservoir = Reservoir::new_seeded(10240, 50000, 42).unwrap();
    let input = vec![0.25; 10240];

    c.bench_function("reservoir_step_50k", |bencher| {
        bencher.iter(|| {
            let state = reservoir.step(black_box(&input)).unwrap();
            black_box(state[0])
        })
    });
}

criterion_group!(
    benches,
    bench_hvec_creation,
    bench_cosine_similarity,
    bench_batch_similarity,
    bench_binding,
    bench_reservoir_step_50k
);
criterion_main!(benches);
