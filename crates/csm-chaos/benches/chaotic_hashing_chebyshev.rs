use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::maps::hyperchaotic::Slhm2d;
use csm_chaos::maps::hyperchaotic_chebyshev::ChebyshevLogistic2d;

fn bench_chaotic_maps(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chaotic Maps Generation");

    group.bench_function("Slhm2d (Baseline)", |b| {
        let mut map = Slhm2d::new(0.123, 0.456, 0.99);
        b.iter(|| {
            black_box(map.next_value());
        });
    });

    group.bench_function("ChebyshevLogistic2d", |b| {
        let mut map = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);
        b.iter(|| {
            black_box(map.next_value());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_chaotic_maps);
criterion_main!(benches);
