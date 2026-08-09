#![allow(clippy::cast_precision_loss)]

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::retrieval::rerank::{MmrReranker, RerankCandidate, Reranker};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;

fn make_candidates(n: usize) -> Vec<RerankCandidate> {
    (0..n)
        .map(|i| RerankCandidate {
            id: format!("c_{i}"),
            vector: Arc::new(HVec10240::random()),
            metadata: HashMap::new(),
            score: 1.0 - (i as f32 / n as f32),
            created_at_unix: 1_700_000_000 + i as u64,
        })
        .collect()
}

fn bench_mmr_rerank(c: &mut Criterion) {
    let query = HVec10240::random();
    let reranker = MmrReranker { lambda: 0.7 };

    let mut group = c.benchmark_group("mmr_rerank");
    for size in [50, 200, 500] {
        let candidates = make_candidates(size);
        group.bench_function(format!("top10_from_{size}"), |b| {
            b.iter_with_setup(
                || candidates.clone(),
                |cands| reranker.rerank(black_box(&query), black_box(cands), 10),
            )
        });
    }
    group.finish();
}

fn bench_mmr_rerank_pure_similarity(c: &mut Criterion) {
    let query = HVec10240::random();
    let reranker = MmrReranker { lambda: 1.0 };

    let mut group = c.benchmark_group("mmr_rerank_pure");
    for size in [50, 200, 500] {
        let candidates = make_candidates(size);
        group.bench_function(format!("top10_from_{size}"), |b| {
            b.iter_with_setup(
                || candidates.clone(),
                |cands| reranker.rerank(black_box(&query), black_box(cands), 10),
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_mmr_rerank, bench_mmr_rerank_pure_similarity);
criterion_main!(benches);
