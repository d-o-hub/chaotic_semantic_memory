//! Unit tests for framework module

use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};

const NS: &str = "_default";

async fn create_framework() -> ChaoticSemanticFramework {
    ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn test_inject_concept_with_metadata_success() {
    let framework = create_framework().await;
    let vector = HVec10240::random();
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("source".to_string(), serde_json::json!("test"));
    metadata.insert("score".to_string(), serde_json::json!(0.95));
    framework
        .inject_concept_with_metadata("concept-1", vector, metadata)
        .await
        .unwrap();
    let concept = framework.get_concept("concept-1").await.unwrap().unwrap();
    assert_eq!(concept.id, "concept-1");
    assert_eq!(concept.metadata.get("source").unwrap(), "test");
    assert_eq!(concept.metadata.get("score").unwrap(), 0.95);
}

#[tokio::test]
async fn test_inject_concept_with_metadata_exceeds_limit() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_metadata_bytes(10)
        .build()
        .await
        .unwrap();
    let vector = HVec10240::random();
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "data".to_string(),
        serde_json::json!("this is a very long string"),
    );
    let result = framework
        .inject_concept_with_metadata("concept-1", vector, metadata)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_framework_builder_with_reservoir_input_size() {
    // Verify builder accepts reservoir config options
    let _framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_size(1000)
        .with_reservoir_input_size(512)
        .with_chaos_strength(0.05)
        .build()
        .await
        .unwrap();
    // Builder succeeds with custom reservoir config
}

#[tokio::test]
async fn test_get_concept_not_found() {
    let framework = create_framework().await;
    assert!(
        framework
            .get_concept("nonexistent")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_inject_concept_with_metadata_empty() {
    let framework = create_framework().await;
    let vector = HVec10240::random();
    framework
        .inject_concept_with_metadata("concept-empty", vector, std::collections::HashMap::new())
        .await
        .unwrap();
    let concept = framework
        .get_concept("concept-empty")
        .await
        .unwrap()
        .unwrap();
    assert!(concept.metadata.is_empty());
}

#[tokio::test]
async fn probe_filtered_with_eq_filter_matches_concept() {
    let fw = create_framework().await;
    let vector = HVec10240::random();
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "type".to_string(),
        serde_json::Value::String("code".to_string()),
    );
    fw.inject_concept_with_metadata("rust".to_string(), vector, metadata)
        .await
        .unwrap();

    let filter = chaotic_semantic_memory::MetadataFilter::Eq(
        "type".to_string(),
        serde_json::Value::String("code".to_string()),
    );
    let results = fw.probe_filtered(&vector, 5, &filter).await.unwrap();
    assert!(
        !results.is_empty(),
        "probe_filtered must return at least one result for matching filter"
    );
    assert!(results.iter().any(|(id, _)| id == "rust"));
}

#[tokio::test]
async fn probe_with_graph_returns_results_for_single_concept() {
    let fw = create_framework().await;
    let vector = HVec10240::random();
    fw.inject_concept("graph".to_string(), vector)
        .await
        .unwrap();
    let config = chaotic_semantic_memory::retrieval::GraphRagConfig {
        anchor_top_k: 5,
        max_hops: 2,
        min_assoc_strength: 0.1,
        similarity_weight: 0.7,
        graph_weight: 0.3,
        final_top_k: 5,
    };
    let results = fw.probe_with_graph(vector, config).await.unwrap();
    assert!(
        !results.is_empty(),
        "probe_with_graph must return at least one result"
    );
}
