use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::maps::hyperchaotic::Slhm2d;
use csm_chaos::maps::svsc_4d::Svsc4d;

fn bench_chaotic_maps(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chaotic Maps Generation");

    group.bench_function("Slhm2d (Baseline)", |b| {
        // Seeds chosen to represent typical chaotic regime in (0, 1)
        let mut map = Slhm2d::new(0.123, 0.456, 0.99);
        b.iter(|| {
            black_box(map.next_value());
        });
    });

    group.bench_function("Svsc4d", |b| {
        // Typical seeds
        let mut map = Svsc4d::new(12345, 67890, 13579, 24680);
        b.iter(|| {
            black_box(map.next_value());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_chaotic_maps);
criterion_main!(benches);
