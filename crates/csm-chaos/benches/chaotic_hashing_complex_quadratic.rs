use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::maps::complex_quadratic::ComplexQuadraticMap;
use csm_chaos::maps::hyperchaotic::Slhm2d;

fn bench_chaotic_maps(c: &mut Criterion) {
    let mut group = c.benchmark_group("Chaotic Maps Generation");

    group.bench_function("Slhm2d (Baseline)", |b| {
        // Seeds chosen to represent typical chaotic regime in (0, 1)
        let mut map = Slhm2d::new(0.123, 0.456, 0.99);
        b.iter(|| {
            black_box(map.next_value());
        });
    });

    group.bench_function("ComplexQuadraticMap", |b| {
        // Seeds chosen near Mandelbrot set boundary
        let mut map = ComplexQuadraticMap::new(0.1, 0.2, -0.8, 0.156);
        b.iter(|| {
            black_box(map.next_value());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_chaotic_maps);
criterion_main!(benches);
