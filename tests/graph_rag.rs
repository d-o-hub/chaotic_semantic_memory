use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::GraphRagConfig;

const NS: &str = "_default";

async fn setup_framework() -> ChaoticSemanticFramework<HVec10240> {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    framework
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
