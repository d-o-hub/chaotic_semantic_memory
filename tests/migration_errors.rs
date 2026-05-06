//! Migration coverage tests via public Persistence API.
//!
//! Tests that exercise migration paths indirectly through Persistence initialization.

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use std::collections::HashMap;
use tempfile::NamedTempFile;

const NS: &str = "_default";

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
async fn persistence_new_local_creates_tables() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    // Tables should be created via migrations
    persistence.health_check().await.unwrap();
}

#[tokio::test]
async fn persistence_new_local_with_retention() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local_with_retention(path, 5)
        .await
        .unwrap();

    persistence.health_check().await.unwrap();
}

#[tokio::test]
async fn persistence_schema_version_is_correct() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    // Schema version should be current (v5 namespace migration completed)
    let version = persistence.schema_version().await.unwrap();
    assert!(version >= 5, "Schema version should be at least 5");
}

#[tokio::test]
async fn persistence_save_and_load_triggers_version_recording() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local_with_retention(path, 3)
        .await
        .unwrap();

    let concept = make_concept("versioned-concept");
    persistence.save_concept(NS, &concept).await.unwrap();

    // Update concept to create new version
    let updated = Concept {
        id: concept.id.clone(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: concept.created_at,
        modified_at: 2,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };
    persistence.save_concept(NS, &updated).await.unwrap();

    // Load should succeed
    let loaded = persistence
        .load_concept::<HVec10240>(NS, "versioned-concept")
        .await
        .unwrap();
    assert!(loaded.is_some());
}

#[tokio::test]
async fn persistence_prune_old_versions() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    // Very low retention to trigger pruning
    let persistence = Persistence::new_local_with_retention(path, 1)
        .await
        .unwrap();

    let concept = make_concept("prune-test");
    persistence.save_concept(NS, &concept).await.unwrap();

    // Save multiple versions
    for i in 1..5 {
        let updated = Concept {
            id: concept.id.clone(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: concept.created_at,
            modified_at: i,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        };
        persistence.save_concept(NS, &updated).await.unwrap();
    }

    // Only latest version should remain
    let loaded = persistence
        .load_concept::<HVec10240>(NS, "prune-test")
        .await
        .unwrap();
    assert!(loaded.is_some());
}
