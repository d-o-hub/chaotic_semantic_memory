//! Integration tests proving that concepts and associations in one namespace
//! are invisible to another namespace.

use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};

// ---------------------------------------------------------------------------
// 1. Default namespace
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_namespace_used_without_specification() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    assert_eq!(fw.namespace(), "_default");
}

#[tokio::test]
async fn custom_namespace_set_via_builder() {
    let fw = ChaoticSemanticFramework::builder()
        .with_namespace("my-ns")
        .without_persistence()
        .build()
        .await
        .unwrap();

    assert_eq!(fw.namespace(), "my-ns");
}

// ---------------------------------------------------------------------------
// 2. Concepts in namespace A are invisible to namespace B's probe
// ---------------------------------------------------------------------------

#[tokio::test]
async fn concepts_invisible_across_namespaces() {
    let fw_alpha = ChaoticSemanticFramework::builder()
        .with_namespace("alpha")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let fw_beta = ChaoticSemanticFramework::builder()
        .with_namespace("beta")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let vec = HVec10240::random();
    fw_alpha.inject_concept("concept-1", vec).await.unwrap();

    // Alpha sees its concept
    let results = fw_alpha.probe(vec, 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "concept-1");

    // Beta cannot see Alpha's concept
    let results = fw_beta.probe(vec, 10).await.unwrap();
    assert!(results.is_empty(), "Beta should not see Alpha's concepts");
}

// ---------------------------------------------------------------------------
// 3. Associations in namespace A don't appear in namespace B
// ---------------------------------------------------------------------------

#[tokio::test]
async fn associations_invisible_across_namespaces() {
    let fw_alpha = ChaoticSemanticFramework::builder()
        .with_namespace("alpha")
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw_alpha
        .inject_concept("node-a", HVec10240::random())
        .await
        .unwrap();
    fw_alpha
        .inject_concept("node-b", HVec10240::random())
        .await
        .unwrap();
    fw_alpha.associate("node-a", "node-b", 0.9).await.unwrap();

    // Alpha has the association
    let assocs = fw_alpha.get_associations("node-a").await.unwrap();
    assert_eq!(assocs.len(), 1);

    // Beta: inject same IDs but in a different namespace
    let fw_beta = ChaoticSemanticFramework::builder()
        .with_namespace("beta")
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw_beta
        .inject_concept("node-a", HVec10240::random())
        .await
        .unwrap();
    fw_beta
        .inject_concept("node-b", HVec10240::random())
        .await
        .unwrap();

    // Beta has no associations for node-a (Alpha's association doesn't leak)
    let assocs = fw_beta.get_associations("node-a").await.unwrap();
    assert!(
        assocs.is_empty(),
        "Beta should not see Alpha's associations"
    );
}

// ---------------------------------------------------------------------------
// 4. Deleting a concept in namespace A doesn't affect namespace B
// ---------------------------------------------------------------------------

#[cfg(feature = "persistence")]
#[tokio::test]
async fn delete_concept_does_not_affect_other_namespace() {
    use tempfile::NamedTempFile;

    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();

    // Framework A with namespace "alpha" and local DB
    let fw_alpha = ChaoticSemanticFramework::builder()
        .with_namespace("alpha")
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    fw_alpha
        .inject_concept("shared-id", HVec10240::random())
        .await
        .unwrap();

    // Framework B with namespace "beta" sharing the same DB
    let fw_beta = ChaoticSemanticFramework::builder()
        .with_namespace("beta")
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    fw_beta
        .inject_concept("shared-id", HVec10240::random())
        .await
        .unwrap();

    // Delete from alpha
    fw_alpha.delete_concept("shared-id").await.unwrap();

    // Alpha no longer has the concept
    let alpha_concept = fw_alpha.get_concept("shared-id").await.unwrap();
    assert!(alpha_concept.is_none());

    // Beta still has its own concept (different namespace)
    let beta_concept = fw_beta.get_concept("shared-id").await.unwrap();
    assert!(
        beta_concept.is_some(),
        "Deleting from Alpha should not affect Beta"
    );
}

// ---------------------------------------------------------------------------
// 5. list_namespaces returns all created namespaces
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_namespaces_returns_created_namespaces() {
    let fw = ChaoticSemanticFramework::builder()
        .with_namespace("test-ns")
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Before injecting any concept, namespace may or may not exist yet
    // (lazy creation). After injecting, it must appear.
    fw.inject_concept("x", HVec10240::random()).await.unwrap();

    let namespaces = fw.list_namespaces().await.unwrap();
    assert!(
        namespaces.contains(&"test-ns".to_string()),
        "Expected 'test-ns' in namespaces, got: {namespaces:?}"
    );
}

#[tokio::test]
async fn list_namespaces_default_after_inject() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Default namespace should appear after data is injected
    fw.inject_concept("y", HVec10240::random()).await.unwrap();

    let namespaces = fw.list_namespaces().await.unwrap();
    assert!(
        namespaces.contains(&"_default".to_string()),
        "Expected '_default' in namespaces, got: {namespaces:?}"
    );
}

// ---------------------------------------------------------------------------
// 6. delete_namespace removes all data for that namespace
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_namespace_removes_all_data() {
    let fw = ChaoticSemanticFramework::builder()
        .with_namespace("doomed")
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("item-1", HVec10240::random())
        .await
        .unwrap();
    fw.inject_concept("item-2", HVec10240::random())
        .await
        .unwrap();
    fw.associate("item-1", "item-2", 0.7).await.unwrap();

    let stats_before = fw.stats().await.unwrap();
    assert_eq!(stats_before.concept_count, 2);

    let deleted = fw.delete_namespace("doomed").await.unwrap();
    assert_eq!(deleted, 2, "Should report 2 deleted concepts");

    // The namespace entry is removed entirely
    let namespaces = fw.list_namespaces().await.unwrap();
    assert!(
        !namespaces.contains(&"doomed".to_string()),
        "'doomed' namespace should be gone after deletion"
    );
}

#[tokio::test]
async fn delete_namespace_does_not_affect_other_namespace() {
    let fw_a = ChaoticSemanticFramework::builder()
        .with_namespace("alpha")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let fw_b = ChaoticSemanticFramework::builder()
        .with_namespace("beta")
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw_a.inject_concept("a1", HVec10240::random())
        .await
        .unwrap();
    fw_b.inject_concept("b1", HVec10240::random())
        .await
        .unwrap();

    // Deleting alpha's namespace should not affect beta
    fw_a.delete_namespace("alpha").await.unwrap();

    let stats_b = fw_b.stats().await.unwrap();
    assert_eq!(
        stats_b.concept_count, 1,
        "Beta should still have its concept after Alpha's namespace is deleted"
    );
}

// ---------------------------------------------------------------------------
// 7. Export/import respects namespace boundaries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn export_import_respects_namespace_boundaries() {
    use tempfile::NamedTempFile;

    // Create framework in namespace "source" and inject data
    let fw_source = ChaoticSemanticFramework::builder()
        .with_namespace("source")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let vec = HVec10240::random();
    fw_source.inject_concept("src-concept", vec).await.unwrap();
    fw_source
        .inject_concept("src-other", HVec10240::random())
        .await
        .unwrap();
    fw_source
        .associate("src-concept", "src-other", 0.6)
        .await
        .unwrap();

    // Export "source" namespace
    let temp = NamedTempFile::new().unwrap();
    let export_path = temp.path().to_str().unwrap();
    fw_source.export_json(export_path).await.unwrap();

    // Import into "target" namespace
    let fw_target = ChaoticSemanticFramework::builder()
        .with_namespace("target")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let count = fw_target.import_json(export_path, false).await.unwrap();
    assert_eq!(count, 2, "Should import 2 concepts");

    // Verify imported data is in "target" namespace
    let target_namespaces = fw_target.list_namespaces().await.unwrap();
    assert!(
        target_namespaces.contains(&"target".to_string()),
        "Imported data should be in 'target' namespace, got: {target_namespaces:?}"
    );
    assert!(
        !target_namespaces.contains(&"source".to_string()),
        "'source' namespace should NOT appear after import into 'target'"
    );

    // Verify the data is accessible in target namespace
    let concept = fw_target.get_concept("src-concept").await.unwrap();
    assert!(
        concept.is_some(),
        "Imported concept should be accessible in target namespace"
    );

    // Verify the association was imported
    let assocs = fw_target.get_associations("src-concept").await.unwrap();
    assert_eq!(assocs.len(), 1, "Association should be imported");
}

#[tokio::test]
async fn export_only_contains_current_namespace_data() {
    use tempfile::NamedTempFile;

    let fw_a = ChaoticSemanticFramework::builder()
        .with_namespace("ns-a")
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw_a.inject_concept("concept-a", HVec10240::random())
        .await
        .unwrap();

    // Export only ns-a data
    let temp = NamedTempFile::new().unwrap();
    let export_path = temp.path().to_str().unwrap();
    fw_a.export_json(export_path).await.unwrap();

    // Import into a fresh framework with a different namespace
    let fw_b = ChaoticSemanticFramework::builder()
        .with_namespace("ns-b")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let count = fw_b.import_json(export_path, false).await.unwrap();
    assert_eq!(
        count, 1,
        "Export should contain exactly 1 concept from ns-a"
    );

    // Verify data is in ns-b, not ns-a
    let namespaces = fw_b.list_namespaces().await.unwrap();
    assert!(
        namespaces.contains(&"ns-b".to_string()),
        "Data should be imported into ns-b"
    );
    assert!(
        !namespaces.contains(&"ns-a".to_string()),
        "ns-a should not appear after importing into ns-b"
    );
}

// ---------------------------------------------------------------------------
// Additional: same concept ID across namespaces
// ---------------------------------------------------------------------------

#[tokio::test]
async fn same_concept_id_coexists_in_different_namespaces() {
    let fw_a = ChaoticSemanticFramework::builder()
        .with_namespace("alpha")
        .without_persistence()
        .build()
        .await
        .unwrap();

    let fw_b = ChaoticSemanticFramework::builder()
        .with_namespace("beta")
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Both inject concepts with the same ID but different vectors
    let vec_a = HVec10240::random();
    let vec_b = HVec10240::random();

    fw_a.inject_concept("shared-id", vec_a).await.unwrap();
    fw_b.inject_concept("shared-id", vec_b).await.unwrap();

    // Each namespace should have its own concept
    let stats_a = fw_a.stats().await.unwrap();
    let stats_b = fw_b.stats().await.unwrap();
    assert_eq!(stats_a.concept_count, 1);
    assert_eq!(stats_b.concept_count, 1);

    // Deleting in alpha should not affect beta
    fw_a.delete_concept("shared-id").await.unwrap();
    assert!(fw_a.get_concept("shared-id").await.unwrap().is_none());
    assert!(
        fw_b.get_concept("shared-id").await.unwrap().is_some(),
        "Beta's concept should remain after Alpha deletes its own"
    );
}
