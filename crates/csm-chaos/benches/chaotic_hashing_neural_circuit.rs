use criterion::{black_box, criterion_group, criterion_main, Criterion};
use csm_chaos::maps::neural_circuit::NeuralCircuitMap;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut map = NeuralCircuitMap::new(0.1, 0.2, 0.3, 2.5, 3.0, 1.5);
    c.bench_function("neural_circuit_next", |b| b.iter(|| map.next_value()));

    let mut group = c.benchmark_group("chaotic_hashing_neural_circuit");
    group.bench_function("generate_10240", |b| {
        let mut map = NeuralCircuitMap::new(0.1, 0.2, 0.3, 2.5, 3.0, 1.5);
        b.iter(|| {
            for _ in 0..10240 {
                black_box(map.next_value());
            }
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
