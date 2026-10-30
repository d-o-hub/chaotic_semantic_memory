use criterion::{Criterion, black_box, criterion_group, criterion_main};
use csm_chaos::maps::hyperchaotic::Slhm2d;
use csm_chaos::maps::svsc_4d::Svsc4d;

fn bench_svsc_4d(c: &mut Criterion) {
    let mut group = c.benchmark_group("chaotic_hashing_svsc_4d_10240");

    group.bench_function("svsc4d_10240_iterations", |b| {
        b.iter(|| {
            let mut map = Svsc4d::new(0.1, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 1.4);
            let mut sum = 0.0;
            for _ in 0..10_240 {
                let (x1, _, _, _) = map.next();
                sum += x1;
            }
            black_box(sum)
        })
    });

    group.bench_function("slhm2d_baseline_10240_iterations", |b| {
        b.iter(|| {
            let mut map = Slhm2d::new(0.1, 0.2, 0.99);
            let mut sum = 0.0;
            for _ in 0..10_240 {
                map.next(); let x = map.x;
                sum += x;
            }
            black_box(sum)
        })
    });

    group.finish();
}

criterion_group!(benches, bench_svsc_4d);
criterion_main!(benches);
