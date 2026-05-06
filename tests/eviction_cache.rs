use chaotic_semantic_memory::prelude::*;
use std::sync::Arc;

const NS: &str = "_default";

#[tokio::test]
async fn eviction_invalidates_query_cache() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_concepts(3)
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    let vec_a = HVec10240::random();
    framework.inject_concept("a", vec_a).await.unwrap();
    framework
        .inject_concept("b", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("c", HVec10240::random())
        .await
        .unwrap();

    let before = framework.probe(vec_a, 3).await.unwrap();
    assert_eq!(before.len(), 3);

    // Inject d → triggers eviction of one concept to stay at limit
    framework
        .inject_concept("d", HVec10240::random())
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);

    // The new concept must exist; one of the originals was evicted
    let after = framework.probe(vec_a, 3).await.unwrap();
    let ids: Vec<&str> = after.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&"d"));
    assert_eq!(after.len(), 3);
}

#[tokio::test]
async fn delete_invalidates_query_cache() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    let query = HVec10240::random();
    framework.inject_concept("x", query).await.unwrap();
    framework
        .inject_concept("y", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("z", HVec10240::random())
        .await
        .unwrap();

    let before = framework.probe(query, 3).await.unwrap();
    assert_eq!(before.len(), 3);

    framework.delete_concept("y").await.unwrap();

    let after = framework.probe(query, 3).await.unwrap();
    let ids: Vec<&str> = after.iter().map(|(id, _)| id.as_str()).collect();
    assert!(!ids.contains(&"y"));
    assert_eq!(after.len(), 2);
}

#[tokio::test]
async fn concurrent_inject_and_probe_no_panic() {
    let framework = Arc::new(
        ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap(),
    );

    let mut handles = Vec::new();

    for i in 0..5 {
        let fw = Arc::clone(&framework);
        handles.push(tokio::spawn(async move {
            fw.inject_concept(format!("inject_{i}"), HVec10240::random())
                .await
                .unwrap();
        }));
    }

    for _ in 0..5 {
        let fw = Arc::clone(&framework);
        handles.push(tokio::spawn(async move {
            let _ = fw.probe(HVec10240::random(), 5).await;
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 5);
}
