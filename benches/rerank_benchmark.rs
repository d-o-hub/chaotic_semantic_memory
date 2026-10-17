#![allow(clippy::cast_precision_loss)]

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::retrieval::rerank::{MmrReranker, RerankCandidate, Reranker};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;

/// Builds `n` candidates. Candidate 0's prefill score is `1.0`, which trips
/// `MmrReranker::rerank`'s high-confidence fast path (`score >= 0.99`), so the
/// `mmr_rerank` and `mmr_rerank_pure` groups measure that path rather than the
/// selection loop. The loop is covered by `bench_mmr_slow_path{,_k1}`.
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

/// Candidates whose prefill scores stay below the `score >= 0.99` fast path, so
/// `MmrReranker::rerank` enters its selection loop.
fn make_loop_candidates(n: usize) -> Vec<RerankCandidate> {
    make_candidates(n)
        .into_iter()
        .enumerate()
        .map(|(i, mut cand)| {
            cand.score = 0.9 - (i as f32 / 10_000.0);
            cand
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

/// The MMR selection loop at the optimized shape (`N = 500`, `top_k = 10`):
/// 4455 pairwise similarities plus 500 query similarities, and 4455 -> 4945
/// when the final-step update is not skipped.
fn bench_mmr_slow_path(c: &mut Criterion) {
    let query = HVec10240::random();
    let reranker = MmrReranker { lambda: 0.7 };

    let mut group = c.benchmark_group("mmr_rerank_slow");
    let size = 500;
    let candidates = make_loop_candidates(size);
    group.bench_function(format!("top10_from_{size}"), |b| {
        b.iter_with_setup(
            || candidates.clone(),
            |cands| reranker.rerank(black_box(&query), black_box(cands), 10),
        )
    });
    group.finish();
}

/// `top_k = 1` probe: there is no second selection, so the whole update pass is
/// dead work (`N - 1` similarities). Its delta isolates the marginal cost of one
/// `cosine_similarity` in this workload.
fn bench_mmr_slow_path_k1(c: &mut Criterion) {
    let query = HVec10240::random();
    let reranker = MmrReranker { lambda: 0.7 };

    let mut group = c.benchmark_group("mmr_rerank_slow_k1");
    let size = 500;
    let candidates = make_loop_candidates(size);
    group.bench_function(format!("top1_from_{size}"), |b| {
        b.iter_with_setup(
            || candidates.clone(),
            |cands| reranker.rerank(black_box(&query), black_box(cands), 1),
        )
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_mmr_rerank,
    bench_mmr_rerank_pure_similarity,
    bench_mmr_slow_path,
    bench_mmr_slow_path_k1
);
criterion_main!(benches);
