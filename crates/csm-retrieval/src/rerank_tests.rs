#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::collections::HashMap;

fn create_candidate(id: &str, score: f32, age_days: f32) -> RerankCandidate {
    let now = csm_memory::unix_now_secs();
    let created_at_unix = now - (age_days * 86400.0) as u64;
    RerankCandidate {
        id: id.to_string(),
        vector: Arc::new(HVec10240::random()),
        metadata: HashMap::new(),
        score,
        created_at_unix,
    }
}

#[test]
fn test_mmr_reranker() {
    let query = HVec10240::zero();
    let v1 = Arc::new(HVec10240::new_seeded(1));
    let v2 = Arc::new(HVec10240::new_seeded(1));
    let v3 = Arc::new(HVec10240::new_seeded(2));

    let c1 = RerankCandidate {
        id: "c1".into(),
        vector: v1,
        metadata: HashMap::new(),
        score: 0.9,
        created_at_unix: 0,
    };
    let c2 = RerankCandidate {
        id: "c2".into(),
        vector: v2,
        metadata: HashMap::new(),
        score: 0.85,
        created_at_unix: 0,
    };
    let c3 = RerankCandidate {
        id: "c3".into(),
        vector: v3,
        metadata: HashMap::new(),
        score: 0.7,
        created_at_unix: 0,
    };

    let results_sim =
        MmrReranker { lambda: 1.0 }.rerank(&query, vec![c1.clone(), c2.clone(), c3.clone()], 2);
    assert_eq!(results_sim[0].id, "c1");
    assert_eq!(results_sim[1].id, "c2");
    assert!((results_sim[0].score - query.cosine_similarity(&c1.vector)).abs() < 1e-6);
    assert!((results_sim[1].score - query.cosine_similarity(&c2.vector)).abs() < 1e-6);

    let results = MmrReranker { lambda: 0.5 }.rerank(&query, vec![c1, c2, c3], 2);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "c1");
    assert_eq!(results[1].id, "c3");
}

#[test]
fn test_recency_reranker() {
    let query = HVec10240::zero();
    let c1 = create_candidate("old", 0.9, 10.0);
    let c2 = create_candidate("new", 0.8, 0.0);

    let reranker = RecencyDecayReranker {
        half_life_days: 5.0,
        blend: 0.5,
    };

    let results = reranker.rerank(&query, vec![c1, c2], 2);
    assert_eq!(results[0].id, "new");
    assert_eq!(results[1].id, "old");
    assert!((results[0].score - 0.9).abs() < 1e-6);
    assert!((results[1].score - 0.575).abs() < 1e-6);
}

#[test]
fn test_parse_rerankers() {
    let rers = parse_rerankers("mmr:0.7,recency:30d:0.8").unwrap();
    assert_eq!(rers.len(), 2);
    assert_eq!(rers[0].name(), "mmr");
    assert_eq!(rers[1].name(), "recency");
}

#[test]
#[cfg(feature = "rerank-cross")]
fn test_parse_rerankers_windows_path() {
    let err = parse_rerankers(r"cross:C:\nonexistent\model.onnx").unwrap_err();
    if let csm_core_lib::error::MemoryError::InvalidInput { reason, .. } = err {
        assert!(reason.contains(r"C:\nonexistent\model.onnx"));
    } else {
        panic!("Expected InvalidInput error with the full path");
    }
}

#[test]
fn test_parse_rerankers_invalid_blend() {
    let err = parse_rerankers("recency:30d:not-a-number").unwrap_err();
    assert!(format!("{err}").contains("invalid recency blend"));
}

#[test]
fn test_recency_top_k_zero_returns_empty() {
    let query = HVec10240::zero();
    let c1 = create_candidate("c1", 0.9, 1.0);
    let reranker = RecencyDecayReranker {
        half_life_days: 5.0,
        blend: 0.5,
    };
    let results = reranker.rerank(&query, vec![c1], 0);
    assert!(
        results.is_empty(),
        "top_k=0 with non-empty candidates must return empty vec"
    );
}

#[test]
fn test_mmr_top_k_zero_returns_empty() {
    let query = HVec10240::zero();
    let c1 = RerankCandidate {
        id: "c1".into(),
        vector: Arc::new(HVec10240::new_seeded(1)),
        metadata: HashMap::new(),
        score: 0.9,
        created_at_unix: 0,
    };
    let reranker = MmrReranker { lambda: 0.5 };
    let results = reranker.rerank(&query, vec![c1], 0);
    assert!(
        results.is_empty(),
        "top_k=0 with non-empty candidates must return empty vec"
    );
}

#[test]
fn test_mmr_lambda_zero_score_is_negative_after_first_selection() {
    let query = HVec10240::zero();
    let v1 = Arc::new(HVec10240::new_seeded(1));
    let v2 = Arc::new(HVec10240::new_seeded(2));

    let sim_v1_v2 = v1.cosine_similarity(&v2);
    assert!(
        sim_v1_v2 > 0.0,
        "seeded vectors must have positive mutual similarity (got {sim_v1_v2})"
    );

    let c1 = RerankCandidate {
        id: "c1".into(),
        vector: v1,
        metadata: HashMap::new(),
        score: 0.9,
        created_at_unix: 0,
    };
    let c2 = RerankCandidate {
        id: "c2".into(),
        vector: v2,
        metadata: HashMap::new(),
        score: 0.8,
        created_at_unix: 0,
    };

    let reranker = MmrReranker { lambda: 0.0 };
    let results = reranker.rerank(&query, vec![c1, c2], 2);
    assert_eq!(results.len(), 2);

    assert!(
        results[0].score <= 0.0,
        "lambda=0 first pick score must be <= 0, got {}",
        results[0].score
    );
    assert!(
        results[1].score < 0.0,
        "lambda=0 second pick score must be < 0 (penalty for similarity to selected), got {}",
        results[1].score
    );
}
