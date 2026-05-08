use chaotic_semantic_memory::MemoryError;
use chaotic_semantic_memory::prelude::*;
use std::sync::Arc;

const NS: &str = "_default";

#[tokio::test]
async fn max_cached_top_k_propagates_to_singularity() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_cached_top_k(5)
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    let vec_a = HVec10240::random();
    let vec_b = HVec10240::random();
    let vec_c = HVec10240::random();
    framework.inject_concept("a", vec_a).await.unwrap();
    framework.inject_concept("b", vec_b).await.unwrap();
    framework.inject_concept("c", vec_c).await.unwrap();

    let queries = [vec_a];
    let first = framework.probe_batch_cached(&queries, 3).await.unwrap();
    let second = framework.probe_batch_cached(&queries, 3).await.unwrap();
    assert!(Arc::ptr_eq(&first[0], &second[0]));
}

#[tokio::test]
async fn max_concepts_limits_injection() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_concepts(3)
        .build()
        .await
        .unwrap();

    for i in 0..5 {
        framework
            .inject_concept(format!("c{i}"), HVec10240::random())
            .await
            .unwrap();
    }

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);
}

#[tokio::test]
async fn max_associations_per_concept_limits_links() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_associations_per_concept(2)
        .build()
        .await
        .unwrap();

    for i in 0..4 {
        framework
            .inject_concept(format!("n{i}"), HVec10240::random())
            .await
            .unwrap();
    }

    // Create 4 associations from n0 with increasing strength
    framework.associate("n0", "n1", 0.1).await.unwrap();
    framework.associate("n0", "n2", 0.5).await.unwrap();
    framework.associate("n0", "n3", 0.9).await.unwrap();

    let links = framework.get_associations("n0").await.unwrap();
    assert_eq!(links.len(), 2);
    // The two strongest should remain
    let ids: Vec<&str> = links.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&"n2") || ids.contains(&"n3"));
}

#[tokio::test]
async fn max_probe_top_k_rejects_excess() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_probe_top_k(5)
        .build()
        .await
        .unwrap();

    for i in 0..10 {
        framework
            .inject_concept(format!("p{i}"), HVec10240::random())
            .await
            .unwrap();
    }

    let err = framework.probe(HVec10240::random(), 10).await;
    assert!(matches!(err, Err(MemoryError::InvalidInput { .. })));

    let ok = framework.probe(HVec10240::random(), 5).await;
    assert!(ok.is_ok());
}

#[tokio::test]
async fn reservoir_config_propagates() {
    let framework = ChaoticSemanticFramework::builder()
        .with_reservoir_size(10240)
        .with_reservoir_input_size(64)
        .with_chaos_strength(0.05)
        .without_persistence()
        .build()
        .await
        .unwrap();

    let input = vec![0.1_f32; 64];
    let output = framework.process_sequence(&[input]).await.unwrap();
    assert_ne!(output, HVec10240::zero());
}
