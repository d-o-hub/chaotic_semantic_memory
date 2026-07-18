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
        b.iter(|| {
            merge_results(
                black_box(&bm25),
                black_box(&hdc),
                black_box(weights),
                black_box(10),
            )
        })
    });
}

fn bench_merge_results_various(c: &mut Criterion) {
    let weights = compute_weights(5);

    for &n in &[100, 1000] {
        for &top_k in &[5, 20, 100] {
            if top_k > n {
                continue;
            }
            let bm25: Vec<(String, f32)> = (0..n)
                .map(|i| (format!("doc_{i}"), 1.0 - i as f32 * (1.0 / n as f32)))
                .collect();
            let offset = n / 2;
            let hdc: Vec<(String, f32)> = (offset..(n + offset))
                .map(|i| {
                    (
                        format!("doc_{i}"),
                        0.9 - (i - offset) as f32 * (0.9 / n as f32),
                    )
                })
                .collect();

            let group_name = format!("merge_results_N{n}_K{top_k}");
            c.bench_function(&group_name, |b| {
                b.iter(|| {
                    merge_results(
                        black_box(&bm25),
                        black_box(&hdc),
                        black_box(weights),
                        black_box(top_k),
                    )
                })
            });
        }
    }
}

criterion_group!(
    benches,
    bench_compute_weights,
    bench_normalize_scores,
    bench_merge_results,
    bench_merge_results_various
);
criterion_main!(benches);
