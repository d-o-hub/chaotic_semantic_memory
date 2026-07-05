use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::unix_now_secs;
use std::collections::HashMap;

#[tokio::test]
async fn inject_concept_with_ttl_sets_expires_at() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let before = unix_now_secs();
    let vector = HVec10240::random();
    framework
        .inject_concept_with_ttl("ttl-concept", vector, 3600)
        .await
        .unwrap();
    let after = unix_now_secs();

    // Verify stored with correct expires_at
    let singularity = framework.singularity();
    let ns = framework.namespace().await;
    let concept = singularity
        .read()
        .await
        .get(&ns, "ttl-concept")
        .expect("concept should exist")
        .clone();
    assert!(concept.expires_at.is_some(), "expires_at should be set");

    let expires_at = concept.expires_at.unwrap();
    // expires_at ≈ now + 3600
    let expected_min = before + 3600;
    let expected_max = after + 3600;
    assert!(
        expires_at >= expected_min && expires_at <= expected_max,
        "expires_at should be between {expected_min} and {expected_max}, got {expires_at}"
    );
}

#[tokio::test]
async fn inject_text_with_ttl_encodes_and_stores() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_text_with_ttl("text-ttl", "hello world", 1800)
        .await
        .unwrap();

    // Verify concept exists with TTL
    let singularity = framework.singularity();
    let ns = framework.namespace().await;
    let concept = singularity
        .read()
        .await
        .get(&ns, "text-ttl")
        .expect("concept should exist")
        .clone();
    assert!(concept.expires_at.is_some(), "expires_at should be set");
}

#[tokio::test]
async fn purge_expired_removes_expired_concepts() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject with very short TTL (1 second)
    let vector = HVec10240::random();
    framework
        .inject_concept_with_ttl("short-ttl", vector, 1)
        .await
        .unwrap();

    // Inject a concept with long TTL (should not be purged)
    framework
        .inject_concept_with_ttl("long-ttl", HVec10240::random(), 3600)
        .await
        .unwrap();

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Purge expired concepts
    let purged = framework.purge_expired().await.unwrap();
    assert!(purged >= 1, "At least one concept should be purged");

    // Verify long-ttl still exists
    let results = framework.probe(HVec10240::random(), 10).await.unwrap();
    let ids: Vec<String> = results.into_iter().map(|(id, _)| id).collect();
    assert!(
        ids.contains(&"long-ttl".to_string()),
        "Long TTL concept should remain"
    );
}

#[tokio::test]
async fn inject_text_with_metadata_no_ttl() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), serde_json::json!("test"));
    metadata.insert("priority".to_string(), serde_json::json!(5));

    framework
        .inject_text_with_metadata("meta-concept", "test content", metadata)
        .await
        .unwrap();

    let singularity = framework.singularity();
    let ns = framework.namespace().await;
    let concept = singularity
        .read()
        .await
        .get(&ns, "meta-concept")
        .expect("concept should exist")
        .clone();
    assert!(
        concept.expires_at.is_none(),
        "inject_text_with_metadata should not set TTL"
    );
    assert_eq!(
        concept.metadata.get("source").unwrap().as_str().unwrap(),
        "test"
    );
    assert_eq!(
        concept.metadata.get("priority").unwrap().as_i64().unwrap(),
        5
    );
}

#[tokio::test]
async fn probe_text_returns_similar_concepts() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_text("doc1", "machine learning algorithms")
        .await
        .unwrap();
    framework
        .inject_text("doc2", "neural network deep learning")
        .await
        .unwrap();
    framework
        .inject_text("doc3", "cooking recipes food")
        .await
        .unwrap();

    let results = framework.probe_text("learning", 5).await.unwrap();
    assert!(!results.is_empty(), "should find similar concepts");

    // Learning-related docs should rank higher than cooking
    let ids: Vec<String> = results.iter().map(|(id, _)| id.clone()).collect();
    assert!(
        ids.contains(&"doc1".to_string()) || ids.contains(&"doc2".to_string()),
        "should find learning-related docs"
    );
}

#[tokio::test]
async fn concept_expires_at_serialization() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let now = unix_now_secs();
    let ttl = 7200;
    let vector = HVec10240::random();

    framework
        .inject_concept_with_ttl("serial-test", vector, ttl)
        .await
        .unwrap();

    let singularity = framework.singularity();
    let ns = framework.namespace().await;
    let concept = singularity
        .read()
        .await
        .get(&ns, "serial-test")
        .expect("concept should exist")
        .clone();

    // Verify expires_at was computed correctly
    let expected_min = now + ttl;
    let expected_max = now + ttl + 1;
    assert!(
        concept.expires_at.unwrap() >= expected_min && concept.expires_at.unwrap() <= expected_max,
        "expires_at should be now + ttl"
    );

    // Verify JSON serialization of expires_at
    let json = serde_json::to_string(&concept).unwrap();
    assert!(
        json.contains("expires_at"),
        "JSON should contain expires_at field"
    );

    // Verify deserialization
    let deserialized: Concept = serde_json::from_str(&json).expect("should deserialize concept");
    assert_eq!(
        deserialized.expires_at, concept.expires_at,
        "expires_at should round-trip"
    );
}

#[tokio::test]
async fn multiple_ttl_concepts_independent_expiry() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject multiple concepts with different TTLs (using larger margins for test stability)
    framework
        .inject_concept_with_ttl("first", HVec10240::random(), 1)
        .await
        .unwrap();
    framework
        .inject_concept_with_ttl("second", HVec10240::random(), 3)
        .await
        .unwrap();
    framework
        .inject_concept_with_ttl("third", HVec10240::random(), 3600)
        .await
        .unwrap();

    // Wait for first concept to expire (1s TTL + margin for injection overhead)
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    let purged = framework.purge_expired().await.unwrap();
    assert_eq!(purged, 1, "only first concept should expire");

    // Wait for second concept to expire (3s TTL)
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let purged = framework.purge_expired().await.unwrap();
    assert_eq!(purged, 1, "second concept should now expire");

    // Third should still exist
    let concept = framework.get_concept("third").await.unwrap();
    assert!(concept.is_some(), "long TTL concept should remain");
}

#[tokio::test]
async fn test_query_in_session_filtering() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject concepts for two different sessions
    let mut meta1 = HashMap::new();
    meta1.insert("session_id".to_string(), serde_json::json!("session-1"));
    framework
        .inject_text_with_metadata("doc-1-1", "apple fruit red", meta1)
        .await
        .unwrap();

    let mut meta2 = HashMap::new();
    meta2.insert("session_id".to_string(), serde_json::json!("session-2"));
    framework
        .inject_text_with_metadata("doc-2-1", "apple fruit green", meta2)
        .await
        .unwrap();

    // Query in session 1
    let results1 = framework
        .query_in_session("apple", "session-1", 10)
        .await
        .unwrap();
    assert_eq!(results1.len(), 1);
    assert_eq!(results1[0].0, "doc-1-1");

    // Query in session 2
    let results2 = framework
        .query_in_session("apple", "session-2", 10)
        .await
        .unwrap();
    assert_eq!(results2.len(), 1);
    assert_eq!(results2[0].0, "doc-2-1");

    // Query in non-existent session
    let results3 = framework
        .query_in_session("apple", "session-3", 10)
        .await
        .unwrap();
    assert!(results3.is_empty());
}

#[tokio::test]
async fn zero_ttl_still_sets_expiration() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Zero TTL should still set expires_at (immediate expiration)
    framework
        .inject_concept_with_ttl("zero-ttl", HVec10240::random(), 0)
        .await
        .unwrap();

    let singularity = framework.singularity();
    let ns = framework.namespace().await;
    let concept = singularity
        .read()
        .await
        .get(&ns, "zero-ttl")
        .unwrap()
        .clone();
    assert!(
        concept.expires_at.is_some(),
        "zero TTL should still set expires_at"
    );
    // expires_at should be approximately now
    let now = unix_now_secs();
    assert!(
        concept.expires_at.unwrap() <= now,
        "zero TTL should expire immediately or already be expired"
    );
}
