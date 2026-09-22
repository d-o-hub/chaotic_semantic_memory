use crate::singularity::{Singularity, SingularityConfig};
use csm_core_lib::hyperdim::HVec10240;

#[test]
fn singularity_last_stats_v2() {
    let s = Singularity::<HVec10240>::new(SingularityConfig::default());
    assert_eq!(s.last_retrieval_stats("_default").candidate_count, 0);
}

#[test]
fn singularity_get_config_v2() {
    let s = Singularity::<HVec10240>::new(SingularityConfig::default());
    assert_eq!(s.retrieval_config().max_candidates, 1000);
}

#[test]
fn test_retrieval_config_for_token_count() {
    use super::RetrievalConfig;

    let short_cfg = RetrievalConfig::for_token_count(2);
    assert_eq!(short_cfg.max_candidates, 200);
    assert!(!short_cfg.enable_graph_candidates);
    assert!(short_cfg.bm25_abort_on_no_overlap);

    let med_cfg = RetrievalConfig::for_token_count(5);
    assert_eq!(med_cfg.max_candidates, 500);
    assert!(med_cfg.enable_graph_candidates);
    assert!(med_cfg.bm25_abort_on_no_overlap);

    let long_cfg = RetrievalConfig::for_token_count(10);
    assert_eq!(long_cfg.max_candidates, 1000);
    assert!(long_cfg.enable_graph_candidates);
    assert!(!long_cfg.bm25_abort_on_no_overlap);
}

#[test]
fn test_retrieval_config_validation() {
    use super::RetrievalConfig;

    let mut cfg = RetrievalConfig {
        early_exit_threshold: Some(1.5),
        ..Default::default()
    };
    assert!(cfg.validate().is_err());

    cfg.early_exit_threshold = Some(0.85);
    assert!(cfg.validate().is_ok());
}

#[test]
fn test_generate_graph_candidates_logic() {
    use super::RetrievalConfig;
    use crate::singularity::ConceptBuilder;
    let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());
    let config = RetrievalConfig {
        enable_graph_candidates: true,
        graph_depth: 1,
        graph_fanout: 2,
        ..Default::default()
    };
    s.set_retrieval_config(config).unwrap();

    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    let v3 = HVec10240::random();
    let v4 = HVec10240::random();

    s.inject(
        "_default",
        ConceptBuilder::new("c1")
            .with_vector(v1)
            .build()
            .unwrap(),
    )
    .unwrap();
    s.inject(
        "_default",
        ConceptBuilder::new("c2").with_vector(v2).build().unwrap(),
    )
    .unwrap();
    s.inject(
        "_default",
        ConceptBuilder::new("c3").with_vector(v3).build().unwrap(),
    )
    .unwrap();
    s.inject(
        "_default",
        ConceptBuilder::new("c4").with_vector(v4).build().unwrap(),
    )
    .unwrap();

    // c1 -> c2 (0.9), c1 -> c3 (0.8), c1 -> c4 (0.1)
    s.associate("_default", "c1", "c2", 0.9).unwrap();
    s.associate("_default", "c1", "c3", 0.8).unwrap();
    s.associate("_default", "c1", "c4", 0.1).unwrap();

    let candidates = s.generate_graph_candidates("_default", &v1);
    // c1 is seed, c2 and c3 are top 2 neighbors. c4 is excluded by fanout=2.
    assert_eq!(candidates.len(), 3);

    let ns_state = s.get_namespace("_default").unwrap();
    let ids: std::collections::HashSet<_> = candidates
        .iter()
        .map(|&idx| ns_state.concept_indices[idx].as_str())
        .collect();
    assert!(ids.contains("c1"));
    assert!(ids.contains("c2"));
    assert!(ids.contains("c3"));
    assert!(!ids.contains("c4"));
}

#[test]
fn bucket_candidates_use_a_multi_probe_and_respect_the_budget() {
    use super::RetrievalConfig;
    use crate::singularity_bucket::BUCKET_BUDGET_SLACK;

    let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());
    // 4096 vectors with distinct first words: with a 2-bit floor mask each bucket
    // holds 1024 members and the multi-probe accepts three buckets (~3072).
    for i in 0..4096usize {
        let mut v = HVec10240::zero();
        v.data[0] = i as u128;
        s.inject(
            "_default",
            crate::singularity::Concept {
                id: format!("c{i}"),
                vector: v,
                metadata: Default::default(),
                created_at: 1,
                modified_at: 1,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            },
        )
        .unwrap();
    }
    let mut query = HVec10240::zero();
    query.data[0] = 0b1010_1010;

    // Budget far below the probe size: the generator declines so the caller
    // scans exactly instead of slicing an oversized bucket by index order.
    let tight = RetrievalConfig {
        enable_bucket_candidates: true,
        bucket_probe_width: 2,
        max_candidates: 64,
        ..RetrievalConfig::default()
    };
    s.set_retrieval_config(tight.clone()).unwrap();
    assert!(
        s.generate_bucket_candidates("_default", &query).is_empty(),
        "an oversized probe must decline instead of returning an index-ordered slice"
    );

    // Enough room: the probe returns the bucket plus its one-bit neighbours,
    // every member within the masked distance of the query.
    let roomy = RetrievalConfig {
        max_candidates: 4096,
        ..tight
    };
    s.set_retrieval_config(roomy).unwrap();
    let candidates = s.generate_bucket_candidates("_default", &query);
    assert!(!candidates.is_empty());
    assert!(
        candidates.len() <= 4096 * BUCKET_BUDGET_SLACK,
        "multi-probe result must stay within the slack budget"
    );
    let ns_state = s.get_namespace("_default").unwrap();
    let mask = 0b11u128;
    for &idx in &candidates {
        let bits = ns_state.concept_vectors[idx].data[0] & mask;
        assert!(
            (bits ^ (query.data[0] & mask)).count_ones() <= 1,
            "every bucket candidate must be within one masked bit of the query"
        );
    }
}

#[test]
fn test_score_specific_candidates_behavior() {
    use crate::singularity::ConceptBuilder;

    let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());
    let vec1 = HVec10240::random();
    let vec2 = HVec10240::random();

    s.inject(
        "_default",
        ConceptBuilder::new("c1")
            .with_vector(vec1)
            .build()
            .unwrap(),
    )
    .unwrap();
    s.inject(
        "_default",
        ConceptBuilder::new("c2")
            .with_vector(vec2)
            .build()
            .unwrap(),
    )
    .unwrap();

    let target_ids = vec!["c1".to_string(), "non_existent".to_string(), "c2".to_string()];
    let scores = s.score_specific_candidates("_default", &vec1, &target_ids);

    assert_eq!(scores.len(), 2, "must score existing concepts only");
    assert_eq!(scores[0].0, "c1");
    // Identical vector comparison produces similarity score 1.0
    assert!((scores[0].1 - 1.0).abs() < 1e-6);
    assert_eq!(scores[1].0, "c2");

    let empty_scores = s.score_specific_candidates("_default", &vec1, &[]);
    assert!(empty_scores.is_empty(), "empty candidates must return empty results");
}
