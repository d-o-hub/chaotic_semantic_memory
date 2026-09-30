use crate::singularity::{Concept, Singularity, SingularityConfig};
use csm_core_lib::hyperdim::HVec10240;
use std::collections::HashMap;

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
fn inserting_a_concept_invalidates_cached_similarity_results() {
    let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());
    let query = HVec10240::new_seeded(2);
    s.inject(
        "_default",
        Concept {
            id: "old".to_string(),
            vector: HVec10240::new_seeded(3),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();

    let first = s.find_similar_cached("_default", &query, 1);
    assert_eq!(first.first().map(|(id, _)| id.as_str()), Some("old"));

    s.inject(
        "_default",
        Concept {
            id: "exact".to_string(),
            vector: query,
            metadata: HashMap::new(),
            created_at: 2,
            modified_at: 2,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();

    let refreshed = s.find_similar_cached("_default", &query, 1);
    assert_eq!(refreshed.first().map(|(id, _)| id.as_str()), Some("exact"));
}

#[test]
fn reinserting_existing_id_does_not_evict_another_concept_at_capacity() {
    let config = SingularityConfig {
        max_concepts: Some(2),
        ..SingularityConfig::default()
    };
    let mut s = Singularity::<HVec10240>::new(config);
    for (id, seed, created_at) in [("oldest", 1, 1), ("existing", 2, 2)] {
        s.inject(
            "_default",
            Concept {
                id: id.to_string(),
                vector: HVec10240::new_seeded(seed),
                metadata: HashMap::new(),
                created_at,
                modified_at: created_at,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            },
        )
        .unwrap();
    }

    s.inject(
        "_default",
        Concept {
            id: "existing".to_string(),
            vector: HVec10240::new_seeded(7),
            metadata: HashMap::new(),
            created_at: 3,
            modified_at: 3,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();

    assert!(s.get("_default", "oldest").is_some());
    assert!(s.get("_default", "existing").is_some());
    assert_eq!(s.len("_default"), 2);
}

#[test]
fn graph_edge_mutations_invalidate_cached_candidates() {
    use crate::singularity::ConceptBuilder;

    let mut s = Singularity::new(SingularityConfig::default());
    s.set_retrieval_config(super::RetrievalConfig {
        enable_graph_candidates: true,
        ..Default::default()
    })
    .unwrap();
    let query = HVec10240::new_seeded(11);
    for (id, vector) in [("seed", query), ("neighbor", HVec10240::new_seeded(12))] {
        s.inject(
            "_default",
            ConceptBuilder::new(id).with_vector(vector).build().unwrap(),
        )
        .unwrap();
    }
    let before = s.find_similar_cached("_default", &query, 2);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].0, "seed");
    assert!(std::sync::Arc::ptr_eq(
        &before,
        &s.find_similar_cached("_default", &query, 2)
    ));

    s.associate("_default", "seed", "neighbor", 0.8).unwrap();
    let connected = s.find_similar_cached("_default", &query, 2);
    assert_eq!(connected.len(), 2);
    assert!(connected.iter().any(|(id, _)| id == "neighbor"));

    s.disassociate("_default", "seed", "neighbor").unwrap();
    let disconnected = s.find_similar_cached("_default", &query, 2);
    assert_eq!(disconnected.len(), 1);
    assert_eq!(disconnected[0].0, "seed");

    s.associate("_default", "seed", "neighbor", 0.8).unwrap();
    let reconnected = s.find_similar_cached("_default", &query, 2);
    assert_eq!(reconnected.len(), 2);
    assert!(reconnected.iter().any(|(id, _)| id == "neighbor"));
    assert!(std::sync::Arc::ptr_eq(
        &reconnected,
        &s.find_similar_cached("_default", &query, 2)
    ));

    s.clear_associations("_default", "seed").unwrap();
    let cleared = s.find_similar_cached("_default", &query, 2);
    assert_eq!(cleared.len(), 1);
    assert_eq!(cleared[0].0, "seed");
}

#[test]
fn score_candidate_positions_matches_owned_wrapper() {
    use crate::singularity::ConceptBuilder;

    let mut s = Singularity::new(SingularityConfig::default());
    for i in 0..4 {
        s.inject(
            "_default",
            ConceptBuilder::new(format!("mem_{i}"))
                .with_vector(HVec10240::new_seeded(i))
                .build()
                .unwrap(),
        )
        .unwrap();
    }

    let query = HVec10240::new_seeded(42);
    let ids: Vec<String> = vec![
        "mem_0".to_string(),
        "missing".to_string(),
        "mem_2".to_string(),
        "mem_3".to_string(),
    ];

    let owned = s.score_specific_candidates("_default", &query, &ids);
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let positions = s.score_candidate_positions("_default", &query, &refs);

    // Same scored set, same order, same similarities; positions index the input.
    assert_eq!(owned.len(), 3);
    assert_eq!(positions.len(), 3);
    assert_eq!(
        positions.iter().map(|&(pos, _)| pos).collect::<Vec<_>>(),
        vec![0, 2, 3]
    );
    for ((id, sim), (pos, pos_sim)) in owned.iter().zip(positions.iter()) {
        assert_eq!(id, &ids[*pos]);
        assert!((sim - pos_sim).abs() < f32::EPSILON);
    }

    assert!(s
        .score_candidate_positions("absent", &query, &refs)
        .is_empty());
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
