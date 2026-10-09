use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::maps::coupled_lorenz::CoupledLorenzMap;
use csm_chaos::maps::hyperchaotic::Slhm2d;

fn bench_coupled_lorenz_batch(c: &mut Criterion) {
    let mut map = CoupledLorenzMap::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5, 0.5, 0.01);
    c.bench_function("coupled_lorenz_batch_10240", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(10240);
            for _ in 0..10240 {
                out.push(map.next_value());
            }
            black_box(out);
        })
    });
}

fn bench_baseline_slhm2d_batch(c: &mut Criterion) {
    let mut map = Slhm2d::new(0.1, 0.2, 0.99);
    c.bench_function("coupled_lorenz_baseline_slhm2d_batch_10240", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(10240);
            for _ in 0..10240 {
                out.push(map.next_value());
            }
            black_box(out);
        })
    });
}

criterion_group!(
    benches,
    bench_coupled_lorenz_batch,
    bench_baseline_slhm2d_batch
);
criterion_main!(benches);
