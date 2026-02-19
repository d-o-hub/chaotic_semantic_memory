use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::reservoir::Reservoir;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

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

fn bench_hvec_bundle(c: &mut Criterion) {
    let mut group = c.benchmark_group("hvec_bundle");

    let vectors_10: Vec<_> = (0..10).map(|_| HVec10240::random()).collect();
    group.bench_function("hvec_bundle_10", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vectors_10)).unwrap())
    });

    let vectors_100: Vec<_> = (0..100).map(|_| HVec10240::random()).collect();
    group.bench_function("hvec_bundle_100", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vectors_100)).unwrap())
    });

    let vectors_1000: Vec<_> = (0..1000).map(|_| HVec10240::random()).collect();
    group.bench_function("hvec_bundle_1000", |b| {
        b.iter(|| HVec10240::bundle(black_box(&vectors_1000)).unwrap())
    });

    group.finish();
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

fn bench_reservoir_to_hypervector(c: &mut Criterion) {
    let mut group = c.benchmark_group("reservoir_to_hypervector");

    let reservoir_1k = Reservoir::new_seeded(1024, 1000, 42).unwrap();
    group.bench_function("1k_error", |bencher| {
        bencher.iter(|| black_box(reservoir_1k.to_hypervector().is_err()))
    });

    let reservoir_10k = Reservoir::new_seeded(10240, 10240, 42).unwrap();
    group.bench_function("10k", |bencher| {
        bencher.iter(|| black_box(reservoir_10k.to_hypervector().unwrap()))
    });

    let reservoir_50k = Reservoir::new_seeded(10240, 50000, 42).unwrap();
    group.bench_function("50k", |bencher| {
        bencher.iter(|| black_box(reservoir_50k.to_hypervector().unwrap()))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hvec_creation,
    bench_cosine_similarity,
    bench_batch_similarity,
    bench_binding,
    bench_hvec_bundle,
    bench_reservoir_step_50k,
    bench_reservoir_to_hypervector
);
criterion_main!(benches);
