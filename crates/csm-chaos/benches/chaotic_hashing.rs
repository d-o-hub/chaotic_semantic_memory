use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::hashing::chaotic_lsh::ChaoticLsh;
use csm_chaos::maps::hyperchaotic::Slhm2d;

fn bench_slhm2d_next(c: &mut Criterion) {
    let mut map = Slhm2d::new(0.1, 0.2, 0.99);
    c.bench_function("slhm2d_next", |b| b.iter(|| map.next_value()));
}

fn bench_chaotic_lsh_project(c: &mut Criterion) {
    let input_dim = 128;
    let lsh = ChaoticLsh::new(0.1, 0.2, 0.99, input_dim);
    let input = vec![1.0f32; input_dim];
    c.bench_function("chaotic_lsh_project_128", |b| {
        b.iter(|| lsh.project(black_box(&input)))
    });
}

fn bench_chaotic_lsh_project_bitwise_parity(c: &mut Criterion) {
    let lsh = ChaoticLsh::new(3.9, 0.1, 0.7, 128);
    let input: Vec<f32> = (0..128).map(|i| (i as f32) * 0.001 - 0.064).collect();
    c.bench_function("chaotic_lsh_project_bitwise_parity", |b| {
        b.iter(|| {
            let scalar = lsh.project_scalar(black_box(&input));
            let simd = lsh.project(black_box(&input));
            assert_eq!(scalar, simd, "SIMD projection must match scalar reference");
        })
    });
}

criterion_group!(
    benches,
    bench_slhm2d_next,
    bench_chaotic_lsh_project,
    bench_chaotic_lsh_project_bitwise_parity
);
criterion_main!(benches);
