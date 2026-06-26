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

criterion_group!(benches, bench_slhm2d_next, bench_chaotic_lsh_project);
criterion_main!(benches);
