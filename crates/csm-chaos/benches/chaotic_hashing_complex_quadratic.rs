use criterion::{Criterion, criterion_group, criterion_main};
use csm_chaos::maps::complex_quadratic::ComplexQuadraticMap;

fn bench_complex_quadratic_next(c: &mut Criterion) {
    let mut map = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);
    c.bench_function("complex_quadratic_next", |b| b.iter(|| map.next_value()));
}

criterion_group!(benches, bench_complex_quadratic_next);
criterion_main!(benches);
