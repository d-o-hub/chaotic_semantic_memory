#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Regression tests for the ADR-0071 reranking pipeline.
//!
//! `probe_with_rerankers` was unreachable between #178 (which dropped its
//! `mod` declaration) and this restoration, so these tests pin both the
//! pipeline wiring and the two behaviours the ADR depends on: the reranker
//! chain must change the returned set, and `initial_top_k < final_top_k` must
//! widen the candidate window instead of under-fetching.

use crate::framework::ChaoticSemanticFramework;
use crate::retrieval::rerank::{MmrReranker, Reranker};
use csm_core_lib::hyperdim::HVec10240;

/// Builds a deterministic fixture: `anchor` is closest to the query,
/// `near_dup` is the second-closest concept but a near-duplicate of `anchor`
/// (only 16 differing bits), and `diverse` is unrelated to both.
///
/// `bind` XORs with a sparse mask, so the distances are exact and stable across
/// runs: `sim(q, anchor) = 0.975`, `sim(q, near_dup) = 0.971875`,
/// `sim(anchor, near_dup) = 0.996875`. The top score stays below 0.99 so the
/// MMR fast path does not swallow the diversity ordering.
fn fixture() -> (HVec10240, HVec10240, HVec10240, HVec10240) {
    let query = HVec10240::new_seeded(11);
    let anchor = {
        let mut mask = HVec10240::zero();
        for bit in 0..128 {
            mask.set_bit(bit);
        }
        query.bind(&mask)
    };
    let near_dup = {
        let mut mask = HVec10240::zero();
        for bit in 0..144 {
            mask.set_bit(bit);
        }
        query.bind(&mask)
    };
    let diverse = HVec10240::new_seeded(99);
    (query, anchor, near_dup, diverse)
}

async fn fixture_framework() -> (ChaoticSemanticFramework, HVec10240, HVec10240, HVec10240) {
    let (query, anchor, near_dup, diverse) = fixture();
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("anchor".to_string(), anchor),
        ("near_dup".to_string(), near_dup),
        ("diverse".to_string(), diverse),
    ])
    .await
    .unwrap();
    (fw, query, anchor, near_dup)
}

#[tokio::test]
async fn probe_with_rerankers_applies_the_mmr_chain() {
    let (fw, query, _, _) = fixture_framework().await;

    // Plain probe ranks by similarity: anchor, then its near-duplicate.
    let plain = fw.probe(query, 3).await.unwrap();
    let plain_ids: Vec<&str> = plain.iter().take(2).map(|(id, _)| id.as_str()).collect();
    assert_eq!(plain_ids, vec!["anchor", "near_dup"]);

    // MMR at lambda = 0.2 trades similarity for diversity, so the second slot
    // must go to the unrelated concept instead of the near-duplicate.
    let reranked = fw
        .probe_with_rerankers(query, 3, &[Box::new(MmrReranker { lambda: 0.2 })], 2)
        .await
        .unwrap();
    let reranked_ids: Vec<&str> = reranked.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(reranked_ids, vec!["anchor", "diverse"]);
}

#[tokio::test]
async fn probe_with_rerankers_widens_the_window_when_final_top_k_is_larger() {
    let (fw, query, _, _) = fixture_framework().await;

    // ADR-0071: the candidate window is max(initial_top_k, final_top_k), so an
    // initial of 1 must not starve a request for 3 results.
    let reranked = fw.probe_with_rerankers(query, 1, &[], 3).await.unwrap();
    assert_eq!(
        reranked.len(),
        3,
        "initial_top_k below final_top_k must widen, not under-fetch"
    );
}

#[tokio::test]
async fn probe_with_rerankers_truncates_to_final_top_k() {
    let (fw, query, _, _) = fixture_framework().await;

    let reranked = fw
        .probe_with_rerankers(query, 3, &[Box::new(MmrReranker { lambda: 1.0 })], 1)
        .await
        .unwrap();
    let ids: Vec<&str> = reranked.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["anchor"],
        "lambda=1.0 keeps pure similarity order"
    );
}

#[test]
fn rerank_reranker_trait_is_object_safe_for_the_pipeline() {
    // The pipeline takes `&[Box<dyn Reranker>]`; keep that contract pinned so a
    // future generic-bound change cannot silently break the public signature.
    let rerankers: Vec<Box<dyn Reranker>> = vec![Box::new(MmrReranker { lambda: 0.5 })];
    assert_eq!(rerankers[0].name(), "mmr");
}
