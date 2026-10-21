#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::export_payload::{ExportConcept, ExportPayload};
use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::hyperdim::HVec10240;

#[tokio::test]
async fn invalid_memory_only_replace_import_preserves_concepts_and_associations() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let first_vector = HVec10240::random();
    let second_vector = HVec10240::random();
    framework
        .inject_concepts(&[
            ("existing-a".to_string(), first_vector),
            ("existing-b".to_string(), second_vector),
        ])
        .await
        .unwrap();
    framework
        .associate("existing-a", "existing-b", 0.6)
        .await
        .unwrap();

    let payload = ExportPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: 0,
        concepts: vec![ExportConcept {
            id: "replacement".to_string(),
            vector: HVec10240::random(),
            metadata: std::collections::HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }],
        associations: vec![("replacement".to_string(), "missing".to_string(), 0.7)],
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-memory-replace.json");
    std::fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();

    assert!(
        framework
            .import_json(path.to_str().unwrap(), false)
            .await
            .is_err()
    );
    assert_eq!(
        framework
            .get_concept("existing-a")
            .await
            .unwrap()
            .unwrap()
            .vector,
        first_vector
    );
    assert_eq!(
        framework
            .get_concept("existing-b")
            .await
            .unwrap()
            .unwrap()
            .vector,
        second_vector
    );
    assert!(
        framework
            .get_concept("replacement")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        framework.get_associations("existing-a").await.unwrap(),
        vec![("existing-b".to_string(), 0.6)]
    );
}
