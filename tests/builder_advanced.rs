//! Framework builder additional configuration coverage tests.
//!
//! Covers: with_reservoir_size, with_reservoir_input_size, with_max_concepts,
//! with_max_associations_per_concept, with_max_probe_top_k, with_max_cached_top_k

use chaotic_semantic_memory::prelude::*;

const NS: &str = "_default";

#[tokio::test]
async fn builder_with_reservoir_size_custom() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_size(10000)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("reservoir-size-test", HVec10240::random())
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 1);
}

#[tokio::test]
async fn builder_with_reservoir_input_size_custom() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_input_size(1024)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("input-size-test", HVec10240::random())
        .await
        .unwrap();
}

#[tokio::test]
async fn builder_with_max_concepts_sets_limit() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_concepts(3)
        .build()
        .await
        .unwrap();

    // Inject concepts up to limit
    for i in 0..3 {
        framework
            .inject_concept(&format!("max-concept-{i}"), HVec10240::random())
            .await
            .unwrap();
    }

    // This should trigger eviction of oldest concept
    framework
        .inject_concept("max-concept-overflow", HVec10240::random())
        .await
        .unwrap();

    // Should still have max_concepts count
    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);
}

#[tokio::test]
async fn builder_with_max_associations_per_concept_sets_limit() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_associations_per_concept(2)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("assoc-limit-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-limit-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-limit-3", HVec10240::random())
        .await
        .unwrap();

    // Create associations up to limit
    framework
        .associate("assoc-limit-1", "assoc-limit-2", 0.5)
        .await
        .unwrap();
    framework
        .associate("assoc-limit-1", "assoc-limit-3", 0.6)
        .await
        .unwrap();

    // This should evict oldest association
    framework
        .inject_concept("assoc-limit-4", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("assoc-limit-1", "assoc-limit-4", 0.7)
        .await
        .unwrap();
}

#[tokio::test]
async fn builder_with_max_probe_top_k_custom() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_probe_top_k(50)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("probe-top-k-test", HVec10240::random())
        .await
        .unwrap();

    // Probe with valid top_k
    let results = framework.probe(HVec10240::random(), 10).await.unwrap();
    assert!(results.len() <= 1);
}

#[tokio::test]
async fn builder_with_max_cached_top_k_custom() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .with_max_cached_top_k(10)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("cached-top-k-test", HVec10240::random())
        .await
        .unwrap();

    // Probe with cached top_k should cache results
    let results = framework
        .probe_batch_cached(&[HVec10240::random()], 5)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn builder_with_max_sequence_length_custom() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_sequence_length(512)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("sequence-length-test", HVec10240::random())
        .await
        .unwrap();
}

#[tokio::test]
async fn builder_with_reservoir_size_invalid_fails() {
    let result = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_size(0) // Invalid: must be > 0
        .build()
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn builder_with_chaos_strength_invalid_fails() {
    let result = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_size(1000)
        .with_chaos_strength(-0.5) // Invalid: negative chaos strength
        .build()
        .await;

    assert!(result.is_err());
}
