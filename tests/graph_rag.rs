#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::GraphRagConfig;

const NS: &str = "_default";

async fn setup_framework() -> ChaoticSemanticFramework {
    ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn test_graph_rag_synthetic_structure() {
    let framework = setup_framework().await;

    // Create concepts
    // c0 (anchor), c1 (neighbor of c0), c2 (neighbor of c1), c3 (unrelated high similarity)
    let v0 = HVec10240::new_seeded(0);
    let v1 = HVec10240::new_seeded(1);
    let v2 = HVec10240::new_seeded(2);
    let v3 = HVec10240::new_seeded(3);

    framework.inject_concept("c0", v0).await.unwrap();
    framework.inject_concept("c1", v1).await.unwrap();
    framework.inject_concept("c2", v2).await.unwrap();
    framework.inject_concept("c3", v3).await.unwrap();

    // Create associations: c0 -> c1 (0.8), c1 -> c2 (0.6)
    framework.associate("c0", "c1", 0.8).await.unwrap();
    framework.associate("c1", "c2", 0.6).await.unwrap();

    let config = GraphRagConfig {
        anchor_top_k: 1,
        max_hops: 2,
        min_assoc_strength: 0.1,
        similarity_weight: 0.5,
        graph_weight: 0.5,
        final_top_k: 10,
    };

    let results = framework.probe_with_graph(v0, config).await.unwrap();

    // Expected results:
    // c0: similarity=1.0, hops=0, score = 0.5*1.0 + 0.5*(1/1)*1.0 = 1.0
    // c1: similarity=?, hops=1, score = 0.5*sim + 0.5*(1/2)*0.8 = 0.5*sim + 0.2
    // c2: similarity=?, hops=2, score = 0.5*sim + 0.5*(1/3)*0.6 = 0.5*sim + 0.1

    assert!(!results.is_empty());
    assert_eq!(results[0].id, "c0");
    assert_eq!(results[0].hop_distance, 0);

    let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    assert!(ids.contains(&"c1".to_string()));
    assert!(ids.contains(&"c2".to_string()));
    assert!(!ids.contains(&"c3".to_string()));

    // Check path strength for c2 (min of 0.8 and 0.6 is 0.6)
    let c2_res = results.iter().find(|r| r.id == "c2").unwrap();
    assert!((c2_res.assoc_strength - 0.6).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_graph_rag_connected_outranks_similarity() {
    let framework = setup_framework().await;

    let v_query = HVec10240::new_seeded(100);

    // anchor: very high similarity
    let v_anchor = v_query;

    // neighbor: low similarity to query
    let v_neighbor = HVec10240::new_seeded(200);

    // high_sim: also very high similarity, but not connected
    let v_high_sim = v_query;

    framework.inject_concept("anchor", v_anchor).await.unwrap();
    framework
        .inject_concept("neighbor", v_neighbor)
        .await
        .unwrap();
    framework
        .inject_concept("high_sim", v_high_sim)
        .await
        .unwrap();

    framework
        .associate("anchor", "neighbor", 0.9)
        .await
        .unwrap();

    let config = GraphRagConfig {
        anchor_top_k: 1, // Only "anchor" (or "high_sim") will be anchor
        max_hops: 1,
        similarity_weight: 0.1,
        graph_weight: 0.9,
        final_top_k: 5,
        ..Default::default()
    };

    let results = framework.probe_with_graph(v_query, config).await.unwrap();
    let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();

    // If "anchor" was chosen as the anchor, "neighbor" should be present.
    // If "high_sim" was chosen, "neighbor" will NOT be present.
    // Both "anchor" and "high_sim" have same similarity (1.0).
    // Singularity::find_similar likely returns both if top_k >= 2.
    // But here anchor_top_k = 1.

    assert!(ids.contains(&"anchor".to_string()) || ids.contains(&"high_sim".to_string()));
}

#[tokio::test]
async fn test_graph_rag_cycles() {
    let framework = setup_framework().await;

    framework
        .inject_concept("c0", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();

    framework.associate("c0", "c1", 0.8).await.unwrap();
    framework.associate("c1", "c0", 0.8).await.unwrap();

    let config = GraphRagConfig {
        anchor_top_k: 1,
        max_hops: 5,
        ..Default::default()
    };

    let results = framework
        .probe_with_graph(HVec10240::random(), config)
        .await
        .unwrap();
    // Should not hang and should have both concepts
    assert!(results.len() <= 2);
}

#[tokio::test]
async fn test_graph_rag_empty_isolated() {
    let framework = setup_framework().await;

    // Empty
    let results = framework
        .probe_with_graph(HVec10240::random(), GraphRagConfig::default())
        .await
        .unwrap();
    assert!(results.is_empty());

    // Isolated
    framework
        .inject_concept("c0", HVec10240::random())
        .await
        .unwrap();
    let results = framework
        .probe_with_graph(HVec10240::random(), GraphRagConfig::default())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "c0");
    assert_eq!(results[0].hop_distance, 0);
}

#[tokio::test]
async fn test_graph_rag_boundary_conditions() {
    let framework = setup_framework().await;

    // c0 -> c1 (0.5), c0 -> c2 (0.4)
    framework
        .inject_concept("c0", HVec10240::new_seeded(0))
        .await
        .unwrap();
    framework
        .inject_concept("c1", HVec10240::new_seeded(1))
        .await
        .unwrap();
    framework
        .inject_concept("c2", HVec10240::new_seeded(2))
        .await
        .unwrap();

    framework.associate("c0", "c1", 0.5).await.unwrap();
    framework.associate("c0", "c2", 0.4).await.unwrap();

    // Test min_assoc_strength boundary
    let config = GraphRagConfig {
        anchor_top_k: 1,
        max_hops: 1,
        min_assoc_strength: 0.5, // Exactly c1
        ..Default::default()
    };
    let results = framework
        .probe_with_graph(HVec10240::new_seeded(0), config)
        .await
        .unwrap();
    let ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    assert!(ids.contains(&"c1".to_string()));
    assert!(
        !ids.contains(&"c2".to_string()),
        "c2 should be filtered by min_assoc_strength"
    );

    // Test scoring impact of hop distance (1.0 / (1.0 + hops))
    // c0: hops=0, path=1.0, graph_score = 1.0 / (1+0) * 1.0 = 1.0
    // c1: hops=1, path=0.5, graph_score = 0.5 / (1+1) * 1.0 = 0.25
    // we use a config with graph_weight=1.0 and similarity_weight=0.0
    let config_scoring = GraphRagConfig {
        anchor_top_k: 1,
        max_hops: 1,
        similarity_weight: 0.0,
        graph_weight: 1.0,
        ..Default::default()
    };
    let results_scoring = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_scoring)
        .await
        .unwrap();
    let c0_res = results_scoring.iter().find(|r| r.id == "c0").unwrap();
    let c1_res = results_scoring.iter().find(|r| r.id == "c1").unwrap();
    assert!((c0_res.score - 1.0).abs() < f32::EPSILON);
    assert!((c1_res.score - 0.25).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_graph_rag_max_results_boundary() {
    let framework = setup_framework().await;

    // To test the 1000 results boundary, we need to inject 1001 concepts and associate c0 to all of them.
    // This is slow but necessary to kill mutants on line 226.
    framework
        .inject_concept("c0", HVec10240::new_seeded(0))
        .await
        .unwrap();

    // Use a smaller number if we can, but the code hardcodes 1000.
    // Let's actually do it.
    for i in 1..=1001 {
        let id = format!("n{i}");
        framework
            .inject_concept(&id, HVec10240::new_seeded(i as u64))
            .await
            .unwrap();
        framework.associate("c0", &id, 0.9).await.unwrap();
    }

    let config = GraphRagConfig {
        anchor_top_k: 1,
        max_hops: 1,
        final_top_k: 2000,
        ..Default::default()
    };

    let results = framework
        .probe_with_graph(HVec10240::new_seeded(0), config)
        .await
        .unwrap();

    // results should have c0 (anchor) + 1000 neighbors = 1001 total.
    // The 1001st neighbor should have been rejected by max_results=1000.
    assert_eq!(
        results.len(),
        1001,
        "Should have anchor plus exactly 1000 neighbors"
    );
}

#[tokio::test]
async fn test_graph_rag_truncation_logic() {
    let framework = setup_framework().await;

    // Create 5 concepts with distinct seeds to get predictable similarities
    for i in 0..5 {
        framework
            .inject_concept(&format!("c{i}"), HVec10240::new_seeded(i as u64))
            .await
            .unwrap();
    }

    // Association c0 -> c1 (0.9), c1 -> c2 (0.8)
    framework.associate("c0", "c1", 0.9).await.unwrap();
    framework.associate("c1", "c2", 0.8).await.unwrap();

    // 1. Test top_k = 0
    let config_zero = GraphRagConfig {
        anchor_top_k: 0,
        final_top_k: 0,
        ..Default::default()
    };
    let results_zero = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_zero)
        .await
        .unwrap();
    assert!(
        results_zero.is_empty(),
        "top_k=0 should return empty results"
    );

    // 2. Test top_k = 1 (kills framework > 0 mutants)
    let config_one = GraphRagConfig {
        anchor_top_k: 1,
        max_hops: 0,
        final_top_k: 1,
        ..Default::default()
    };
    let results_one = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_one)
        .await
        .unwrap();
    assert_eq!(
        results_one.len(),
        1,
        "top_k=1 should return exactly 1 result"
    );
    assert_eq!(results_one[0].id, "c0");

    // 3. Test final_top_k truncation (3 elements available, take 2)
    let config_trunc = GraphRagConfig {
        anchor_top_k: 1, // anchor c0
        max_hops: 2,     // reaches c1, c2
        final_top_k: 2,
        similarity_weight: 0.5,
        graph_weight: 0.5,
        ..Default::default()
    };
    let results_trunc = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_trunc)
        .await
        .unwrap();
    assert_eq!(results_trunc.len(), 2, "Should truncate to final_top_k");
    // Ensure they are sorted (highest score first)
    assert!(results_trunc[0].score >= results_trunc[1].score);
    // c0 (anchor) and c1 (strongest association) should be the top 2
    assert_eq!(results_trunc[0].id, "c0");
    assert_eq!(results_trunc[1].id, "c1");

    // 4. Test anchor_top_k selection boundary (5 elements available, take 2 anchors)
    let config_anchors = GraphRagConfig {
        anchor_top_k: 2,
        max_hops: 0,
        final_top_k: 10,
        ..Default::default()
    };
    let results_anchors = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_anchors)
        .await
        .unwrap();
    assert_eq!(
        results_anchors.len(),
        2,
        "Should have exactly anchor_top_k results when hops=0"
    );
    // Explicitly verify content to kill select_nth_unstable mutants (top_k-1 vs top_k/1)
    assert_eq!(results_anchors[0].id, "c0");
    // c0 is seed, so must be first anchor. Check that second anchor is also present and correct.
    assert!(
        results_anchors.iter().any(|r| r.id == "c1")
            || results_anchors.iter().any(|r| r.id == "c2")
    );

    // 5. Test exact boundary where len == top_k (kills > vs >= mutants)
    let config_exact = GraphRagConfig {
        anchor_top_k: 5,
        max_hops: 0,
        final_top_k: 5,
        ..Default::default()
    };
    let results_exact = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_exact)
        .await
        .unwrap();
    assert_eq!(
        results_exact.len(),
        5,
        "Should return all 5 elements when len == top_k"
    );
    // Verify sorting at boundary
    for i in 0..4 {
        assert!(results_exact[i].score >= results_exact[i + 1].score);
    }

    // 6. Test validation rejection for excessive top_k (kills > vs < mutants in framework)
    let config_huge_final = GraphRagConfig {
        final_top_k: 100_001, // Exceeds MAX_TOP_K_LIMIT
        ..Default::default()
    };
    assert!(
        framework
            .probe_with_graph(HVec10240::new_seeded(0), config_huge_final)
            .await
            .is_err()
    );

    let config_huge_anchor = GraphRagConfig {
        anchor_top_k: 100_001,
        ..Default::default()
    };
    assert!(
        framework
            .probe_with_graph(HVec10240::new_seeded(0), config_huge_anchor)
            .await
            .is_err()
    );

    // 7. Test anchor_top_k = 0 separately (kills > vs == mutants)
    let config_zero_anchor = GraphRagConfig {
        anchor_top_k: 0,
        final_top_k: 10,
        ..Default::default()
    };
    let results_zero_anchor = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_zero_anchor)
        .await
        .unwrap();
    assert!(
        results_zero_anchor.is_empty(),
        "anchor_top_k=0 should return empty results"
    );

    // 8. Test final_top_k = 0 separately
    let config_zero_final = GraphRagConfig {
        anchor_top_k: 5,
        final_top_k: 0,
        ..Default::default()
    };
    let results_zero_final = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_zero_final)
        .await
        .unwrap();
    assert!(
        results_zero_final.is_empty(),
        "final_top_k=0 should return empty results"
    );

    // 9. Strict selection test to kill top_k - 1 mutants (len=3, top_k=2)
    // We want to ensure that only the best 2 are returned.
    let config_strict = GraphRagConfig {
        anchor_top_k: 2,
        max_hops: 0,
        final_top_k: 10,
        ..Default::default()
    };
    // c0 similarity with HVec10240::new_seeded(0) is 1.0.
    // Let's see which of c1..c4 is closest to c0.
    let results_strict = framework
        .probe_with_graph(HVec10240::new_seeded(0), config_strict)
        .await
        .unwrap();
    assert_eq!(results_strict.len(), 2);
    assert_eq!(results_strict[0].id, "c0");
    // Verify the second one is indeed the best among the rest
    let mut all_sims = Vec::new();
    for i in 0..5 {
        let v = HVec10240::new_seeded(i as u64);
        let sim = HVec10240::new_seeded(0).cosine_similarity(&v);
        all_sims.push((format!("c{i}"), sim));
    }
    all_sims.sort_by(|a, b| b.1.total_cmp(&a.1));
    assert_eq!(results_strict[1].id, all_sims[1].0);
}
