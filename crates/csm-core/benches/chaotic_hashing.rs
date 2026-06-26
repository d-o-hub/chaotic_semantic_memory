#[cfg(feature = "chaotic-hashing")]
use criterion::{Criterion, black_box, criterion_group, criterion_main};
#[cfg(feature = "chaotic-hashing")]
use csm_core::hashing::chaotic_lsh::ChaoticLsh;
#[cfg(feature = "chaotic-hashing")]
use csm_core::maps::hyperchaotic::Slhm2d;

#[cfg(feature = "chaotic-hashing")]
fn bench_slhm2d_next(c: &mut Criterion) {
    let mut map = Slhm2d::new(0.1, 0.2, 0.99);
    c.bench_function("slhm2d_next", |b| b.iter(|| map.next_value()));
}

#[cfg(feature = "chaotic-hashing")]
fn bench_chaotic_lsh_project(c: &mut Criterion) {
    let input_dim = 128;
    let lsh = ChaoticLsh::new(0.1, 0.2, 0.99, input_dim);
    let input = vec![1.0f32; input_dim];
    c.bench_function("chaotic_lsh_project_128", |b| {
        b.iter(|| lsh.project(black_box(&input)))
    });
}

#[cfg(feature = "chaotic-hashing")]
criterion_group!(benches, bench_slhm2d_next, bench_chaotic_lsh_project);
#[cfg(feature = "chaotic-hashing")]
criterion_main!(benches);

#[cfg(not(feature = "chaotic-hashing"))]
fn main() {}
