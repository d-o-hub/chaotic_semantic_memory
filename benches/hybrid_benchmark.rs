#![allow(clippy::cast_precision_loss)]

use chaotic_semantic_memory::retrieval::hybrid::{
    compute_weights, merge_results, normalize_scores,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_compute_weights(c: &mut Criterion) {
    c.bench_function("compute_weights", |b| {
        b.iter(|| {
            for n in [1, 3, 5, 9, 15] {
                black_box(compute_weights(n));
            }
        })
    });
}

fn bench_normalize_scores(c: &mut Criterion) {
    let scores: Vec<(String, f32)> = (0..1000)
        .map(|i| (format!("doc_{i}"), i as f32 * 0.001))
        .collect();

    c.bench_function("normalize_scores_1000", |b| {
        b.iter(|| normalize_scores(black_box(&scores)))
    });
}

fn bench_merge_results(c: &mut Criterion) {
    let bm25: Vec<(String, f32)> = (0..500)
        .map(|i| (format!("doc_{i}"), 1.0 - i as f32 * 0.002))
        .collect();
    let hdc: Vec<(String, f32)> = (250..750)
        .map(|i| (format!("doc_{i}"), 0.9 - (i - 250) as f32 * 0.0018))
        .collect();
    let weights = compute_weights(5);

    c.bench_function("merge_results_500x500", |b| {
        b.iter(|| merge_results(black_box(&bm25), black_box(&hdc), black_box(weights)))
    });
}

criterion_group!(
    benches,
    bench_compute_weights,
    bench_normalize_scores,
    bench_merge_results
);
criterion_main!(benches);
