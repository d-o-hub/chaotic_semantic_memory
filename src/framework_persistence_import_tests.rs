#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::hyperdim::HVec10240;
use tempfile::tempdir;

#[tokio::test]
async fn replacement_import_commits_concepts_and_associations_together() {
    use crate::export_payload::{ExportConcept, ExportPayload};

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("replacement-import.db");
    let db = db_path.to_str().unwrap();
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    fw.inject_concept("old", HVec10240::random()).await.unwrap();

    let payload = ExportPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: 0,
        concepts: ["a", "b"]
            .into_iter()
            .map(|id| ExportConcept {
                id: id.to_string(),
                vector: HVec10240::random(),
                metadata: std::collections::HashMap::new(),
                created_at: 0,
                modified_at: 0,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            })
            .collect(),
        associations: vec![("a".to_string(), "b".to_string(), 0.8)],
    };
    let import_path = dir.path().join("replacement.json");
    std::fs::write(&import_path, serde_json::to_vec(&payload).unwrap()).unwrap();

    assert_eq!(
        fw.import_json(import_path.to_str().unwrap(), false)
            .await
            .unwrap(),
        2
    );
    assert!(fw.get_concept("old").await.unwrap().is_none());
    assert_eq!(
        fw.get_associations("a").await.unwrap(),
        vec![("b".to_string(), 0.8)]
    );

    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    reloaded.load().await.unwrap();
    assert_eq!(
        reloaded.get_associations("a").await.unwrap(),
        vec![("b".to_string(), 0.8)]
    );
    assert!(reloaded.get_concept("old").await.unwrap().is_none());
}
