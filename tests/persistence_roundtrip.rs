use std::collections::HashMap;

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use tempfile::NamedTempFile;

#[tokio::test]
async fn persistence_roundtrip_crud() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = Concept {
        id: "alpha".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
    };

    persistence.save_concept(&concept).await.unwrap();
    persistence
        .save_association("alpha", "alpha", 0.5)
        .await
        .unwrap();

    let loaded = persistence.load_concept("alpha").await.unwrap();
    assert!(loaded.is_some());

    let associations = persistence.load_associations("alpha").await.unwrap();
    assert_eq!(associations.len(), 1);

    persistence.delete_concept("alpha").await.unwrap();
    let missing = persistence.load_concept("alpha").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn persistence_rejects_association_for_missing_concept_when_fk_enabled() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = Concept {
        id: "alpha".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
    };
    persistence.save_concept(&concept).await.unwrap();

    let result = persistence.save_association("alpha", "missing", 0.5).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn persistence_health_check_and_schema_version_work() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence.health_check().await.unwrap();
    let schema_version = persistence.schema_version().await.unwrap();
    assert!(schema_version >= 2);
}

#[tokio::test]
async fn backup_and_restore_roundtrip_state() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path().to_str().unwrap();
    let backup = NamedTempFile::new().unwrap();
    let backup_path = backup.path().to_str().unwrap();

    let persistence = Persistence::new_local(db_path).await.unwrap();
    let concept_alpha = Concept {
        id: "alpha".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
    };
    persistence.save_concept(&concept_alpha).await.unwrap();
    persistence.backup(backup_path).await.unwrap();

    let concept_beta = Concept {
        id: "beta".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 2,
        modified_at: 2,
        expires_at: None,
    };
    persistence.save_concept(&concept_beta).await.unwrap();

    persistence.restore(backup_path).await.unwrap();
    let alpha = persistence.load_concept("alpha").await.unwrap();
    let beta = persistence.load_concept("beta").await.unwrap();
    assert!(alpha.is_some());
    assert!(beta.is_none());
}
