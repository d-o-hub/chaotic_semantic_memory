//! Framework validation error branch coverage tests.
//!
//! Covers: concept ID validation (empty, too long, control chars),
//! association strength validation (non-finite, negative),
//! metadata size limit exceeded

use chaotic_semantic_memory::prelude::*;

const NS: &str = "_default";

#[tokio::test]
async fn concept_id_empty_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.inject_concept("", HVec10240::random()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn concept_id_too_long_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Create an ID longer than MAX_CONCEPT_ID_BYTES (256)
    let long_id = "x".repeat(300);
    let result = framework
        .inject_concept(&long_id, HVec10240::random())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn concept_id_control_characters_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Create an ID with control characters
    let bad_id = "test\u{0001}id"; // Contains control character
    let result: Result<()> = framework.inject_concept(bad_id, HVec10240::random()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn association_strength_negative_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("assoc-neg-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-neg-2", HVec10240::random())
        .await
        .unwrap();

    let result = framework
        .associate("assoc-neg-1", "assoc-neg-2", -0.5)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn association_strength_infinite_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("assoc-inf-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-inf-2", HVec10240::random())
        .await
        .unwrap();

    let result = framework
        .associate("assoc-inf-1", "assoc-inf-2", f32::INFINITY)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn metadata_size_exceeded_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_metadata_bytes(10) // Very small limit
        .build()
        .await
        .unwrap();

    // Create metadata that exceeds the limit
    let large_metadata = std::collections::HashMap::from([
        ("key1".to_string(), serde_json::json!("value1")),
        ("key2".to_string(), serde_json::json!("value2")),
    ]);

    let result = framework
        .inject_concept_with_metadata("meta-limit", HVec10240::random(), large_metadata)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn top_k_zero_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("topk-test", HVec10240::random())
        .await
        .unwrap();

    let result = framework.probe(HVec10240::random(), 0).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn batch_size_exceeded_fails() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_batch_size(2)
        .build()
        .await
        .unwrap();

    // Create a batch larger than the limit
    let concepts: Vec<(String, HVec10240)> = (0..10)
        .map(|i| (format!("batch-{i}"), HVec10240::random()))
        .collect();

    let result = framework.inject_concepts(&concepts).await;
    assert!(result.is_err());
}
