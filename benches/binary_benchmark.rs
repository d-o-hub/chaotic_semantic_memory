use chaotic_semantic_memory::BHVec10240;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_bhvec_bundle(c: &mut Criterion) {
    let vectors: Vec<BHVec10240> = (0..10).map(|_| BHVec10240::random()).collect();
    let vector_refs: Vec<&BHVec10240> = vectors.iter().collect();

    c.bench_function("bhvec_bundle_10", |b| {
        b.iter(|| BHVec10240::bundle(black_box(&vector_refs)))
    });
}

fn bench_bhvec_hamming(c: &mut Criterion) {
    let v1 = BHVec10240::random();
    let v2 = BHVec10240::random();

    c.bench_function("bhvec_hamming", |b| {
        b.iter(|| black_box(&v1).hamming(black_box(&v2)))
    });
}

criterion_group!(benches, bench_bhvec_bundle, bench_bhvec_hamming);
criterion_main!(benches);
