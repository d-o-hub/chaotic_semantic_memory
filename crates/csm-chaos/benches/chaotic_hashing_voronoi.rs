#![allow(clippy::cast_precision_loss)]
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::hashing::{chaotic_lsh::ChaoticLsh, voronoi_lsh::VoronoiLsh};

fn bench_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("chaotic_hashing");

    let input_dim = 1536; // Typical embedding size
    let input: Vec<f32> = (0..input_dim).map(|i| (i as f32) / 1000.0).collect();

    let chaotic = ChaoticLsh::new(0.5, 0.5, 3.99, input_dim);
    let voronoi = VoronoiLsh::new(0.5, 0.5, 3.99, input_dim);

    group.bench_function("chaotic_lsh_1536", |b| {
        b.iter(|| chaotic.project(black_box(&input)))
    });

    group.bench_function("voronoi_lsh_1536", |b| {
        b.iter(|| voronoi.project(black_box(&input)))
    });

    group.finish();
}

criterion_group!(benches, bench_hashing);
criterion_main!(benches);
