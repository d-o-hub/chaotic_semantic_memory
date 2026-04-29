//! Additional persistence operations tests for coverage gap.
//!
//! Covers: persistence_ops.rs error paths, association operations

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use std::collections::HashMap;
use tempfile::NamedTempFile;

fn make_concept(id: &str) -> Concept {
    Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

#[tokio::test]
async fn persistence_save_concept_overwrites_existing() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = make_concept("overwrite-test");
    persistence.save_concept(&concept).await.unwrap();

    // Update with different vector
    let updated = Concept {
        id: "overwrite-test".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 2,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };
    persistence.save_concept(&updated).await.unwrap();

    let loaded = persistence.load_concept("overwrite-test").await.unwrap();
    assert!(loaded.is_some());
}

#[tokio::test]
async fn persistence_delete_nonexistent_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    // Delete concept that doesn't exist - should succeed
    persistence.delete_concept("nonexistent").await.unwrap();
}

#[tokio::test]
async fn persistence_association_lifecycle() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    // Create concepts first
    persistence
        .save_concept(&make_concept("from-id"))
        .await
        .unwrap();
    persistence
        .save_concept(&make_concept("to-id"))
        .await
        .unwrap();

    // Save association
    persistence
        .save_association("from-id", "to-id", 0.8)
        .await
        .unwrap();

    // Load associations (returns Vec<(String, f32)>)
    let assocs = persistence.load_associations("from-id").await.unwrap();
    assert_eq!(assocs.len(), 1);
    assert_eq!(assocs[0].0, "to-id");
    assert_eq!(assocs[0].1, 0.8);

    // Delete association
    persistence
        .delete_association("from-id", "to-id")
        .await
        .unwrap();

    let assocs_after = persistence.load_associations("from-id").await.unwrap();
    assert!(assocs_after.is_empty());
}

#[tokio::test]
async fn persistence_clear_associations() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(&make_concept("clear-from"))
        .await
        .unwrap();
    persistence
        .save_concept(&make_concept("clear-to1"))
        .await
        .unwrap();
    persistence
        .save_concept(&make_concept("clear-to2"))
        .await
        .unwrap();

    persistence
        .save_association("clear-from", "clear-to1", 0.5)
        .await
        .unwrap();
    persistence
        .save_association("clear-from", "clear-to2", 0.6)
        .await
        .unwrap();

    // Clear all associations
    persistence
        .clear_concept_associations("clear-from")
        .await
        .unwrap();

    let assocs = persistence.load_associations("clear-from").await.unwrap();
    assert!(assocs.is_empty());
}

#[tokio::test]
async fn persistence_list_all_concepts() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(&make_concept("list-1"))
        .await
        .unwrap();
    persistence
        .save_concept(&make_concept("list-2"))
        .await
        .unwrap();
    persistence
        .save_concept(&make_concept("list-3"))
        .await
        .unwrap();

    let concepts = persistence.load_all_concepts().await.unwrap();
    assert_eq!(concepts.len(), 3);
}

#[tokio::test]
async fn persistence_load_nonexistent_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let result = persistence.load_concept("does-not-exist").await.unwrap();
    assert!(result.is_none());
}
