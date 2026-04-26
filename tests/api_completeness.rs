//! Tests for Phase 33 API completeness features.

use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};
use std::collections::HashMap;

async fn create_test_framework() -> ChaoticSemanticFramework {
    ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn test_update_concept_vector() {
    let framework = create_test_framework().await;
    let id = "test-concept";
    let vector1 = HVec10240::random();
    let vector2 = HVec10240::random();

    // Inject concept
    framework.inject_concept(id, vector1).await.unwrap();
    let sing = framework.singularity();
    let guard = sing.read().await;
    let concept = guard.get(id).unwrap();
    assert_eq!(concept.vector, vector1);
    drop(guard);

    // Update vector
    framework.update_concept_vector(id, vector2).await.unwrap();
    let sing = framework.singularity();
    let guard = sing.read().await;
    let concept = guard.get(id).unwrap();
    assert_eq!(concept.vector, vector2);
}

#[tokio::test]
async fn test_update_concept_vector_not_found() {
    let framework = create_test_framework().await;
    let vector = HVec10240::random();

    let result = framework.update_concept_vector("nonexistent", vector).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_concept_metadata() {
    let framework = create_test_framework().await;
    let id = "test-concept";
    let vector = HVec10240::random();

    // Inject concept
    framework.inject_concept(id, vector).await.unwrap();

    // Update metadata
    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), serde_json::json!("value"));
    framework
        .update_concept_metadata(id, metadata.clone())
        .await
        .unwrap();

    let sing = framework.singularity();
    let guard = sing.read().await;
    let concept = guard.get(id).unwrap();
    assert_eq!(
        concept.metadata.get("key").unwrap(),
        &serde_json::json!("value")
    );
}

#[tokio::test]
async fn test_disassociate() {
    let framework = create_test_framework().await;
    let id1 = "concept-1";
    let id2 = "concept-2";

    framework
        .inject_concept(id1, HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept(id2, HVec10240::random())
        .await
        .unwrap();
    framework.associate(id1, id2, 0.8).await.unwrap();

    // Verify association exists
    let sing = framework.singularity();
    let associations = sing.read().await.get_associations(id1);
    assert_eq!(associations.len(), 1);

    // Disassociate
    framework.disassociate(id1, id2).await.unwrap();

    // Verify association removed
    let associations = framework.singularity().read().await.get_associations(id1);
    assert!(associations.is_empty());
}

#[tokio::test]
async fn test_clear_associations() {
    let framework = create_test_framework().await;
    let id1 = "concept-1";
    let id2 = "concept-2";
    let id3 = "concept-3";

    framework
        .inject_concept(id1, HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept(id2, HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept(id3, HVec10240::random())
        .await
        .unwrap();
    framework.associate(id1, id2, 0.8).await.unwrap();
    framework.associate(id1, id3, 0.6).await.unwrap();

    // Verify associations exist
    let sing = framework.singularity();
    let associations = sing.read().await.get_associations(id1);
    assert_eq!(associations.len(), 2);

    // Clear all associations
    framework.clear_associations(id1).await.unwrap();

    // Verify all associations removed
    let associations = framework.singularity().read().await.get_associations(id1);
    assert!(associations.is_empty());
}

#[tokio::test]
async fn test_bundle_concepts_strict() {
    let framework = create_test_framework().await;
    let id1 = "concept-1";
    let id2 = "concept-2";

    framework
        .inject_concept(id1, HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept(id2, HVec10240::random())
        .await
        .unwrap();

    // Bundle existing concepts
    let result = framework
        .bundle_concepts_strict(&[id1.to_string(), id2.to_string()])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_bundle_concepts_strict_missing() {
    let framework = create_test_framework().await;
    let id1 = "concept-1";

    framework
        .inject_concept(id1, HVec10240::random())
        .await
        .unwrap();

    // Bundle with missing concept
    let result = framework
        .bundle_concepts_strict(&[id1.to_string(), "nonexistent".to_string()])
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_clear_similarity_cache() {
    let framework = create_test_framework().await;
    let id = "concept-1";
    let vector = HVec10240::random();

    framework.inject_concept(id, vector).await.unwrap();

    // Query to populate cache
    let _ = framework.probe(vector, 10).await.unwrap();

    // Clear cache (should not error)
    framework.clear_similarity_cache().await;
}
