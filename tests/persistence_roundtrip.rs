use std::collections::HashMap;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use chaotic_semantic_memory::HVec10240;
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
    };
    persistence.save_concept(&concept).await.unwrap();

    let result = persistence.save_association("alpha", "missing", 0.5).await;
    assert!(result.is_err());
}
