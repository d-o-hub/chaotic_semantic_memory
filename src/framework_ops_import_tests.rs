#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::export_payload::{ExportConcept, ExportPayload};
use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::error::MemoryError;
use csm_core_lib::hyperdim::HVec10240;

fn export_concept(id: &str) -> ExportConcept {
    ExportConcept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: std::collections::HashMap::new(),
        created_at: 0,
        modified_at: 0,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

fn payload(
    concepts: Vec<ExportConcept>,
    associations: Vec<(String, String, f32)>,
) -> ExportPayload {
    ExportPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: 0,
        concepts,
        associations,
    }
}

/// `apply_import_payload` rejects a payload before it touches memory, so each
/// rejection test below asserts the exact error variant *and* the untouched
/// state: a guard whose removal still produces an `Err` (from a later `inject`)
/// must not pass. `tests/**` cannot kill those mutants — the mutation gate runs
/// `--lib` only — so these assertions live in the lib target.
fn write_payload(dir: &std::path::Path, name: &str, payload: &ExportPayload) -> String {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec(payload).unwrap()).unwrap();
    path.to_str().unwrap().to_string()
}

async fn memory_only_framework() -> ChaoticSemanticFramework {
    ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn invalid_memory_only_replace_import_preserves_concepts_and_associations() {
    let framework = memory_only_framework().await;
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

    let payload = payload(
        vec![export_concept("replacement")],
        vec![("replacement".to_string(), "missing".to_string(), 0.7)],
    );
    let directory = tempfile::tempdir().unwrap();
    let path = write_payload(directory.path(), "invalid-memory-replace.json", &payload);

    assert!(
        framework.import_json(&path, false).await.is_err(),
        "an association naming a concept that is neither imported nor live must reject the import"
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

#[tokio::test]
async fn duplicate_concept_id_rejects_the_import_with_the_validation_error() {
    let framework = memory_only_framework().await;
    let live_vector = HVec10240::random();
    framework
        .inject_concepts(&[("live".to_string(), live_vector)])
        .await
        .unwrap();

    let payload = payload(
        vec![export_concept("twice"), export_concept("twice")],
        Vec::new(),
    );
    let directory = tempfile::tempdir().unwrap();
    let path = write_payload(directory.path(), "duplicate-concept.json", &payload);

    let error = framework
        .import_json(&path, false)
        .await
        .expect_err("the same id twice in one payload is not a last-write-wins import");
    assert!(
        matches!(
            error,
            MemoryError::InvalidInput { ref field, ref reason }
                if field == "import_data" && reason.contains("duplicate concept id: twice")
        ),
        "unexpected error: {error:?}"
    );

    assert_eq!(
        framework.get_concept("live").await.unwrap().unwrap().vector,
        live_vector
    );
    assert!(framework.get_concept("twice").await.unwrap().is_none());
}

#[tokio::test]
async fn duplicate_association_edge_rejects_the_import_with_the_validation_error() {
    let framework = memory_only_framework().await;
    framework
        .inject_concepts(&[("live".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let payload = payload(
        vec![export_concept("a"), export_concept("b")],
        vec![
            ("a".to_string(), "b".to_string(), 0.5),
            ("a".to_string(), "b".to_string(), 0.9),
        ],
    );
    let directory = tempfile::tempdir().unwrap();
    let path = write_payload(directory.path(), "duplicate-association.json", &payload);

    let error = framework
        .import_json(&path, false)
        .await
        .expect_err("a repeated edge must reject the payload, not silently keep one strength");
    assert!(
        matches!(
            error,
            MemoryError::InvalidInput { ref field, ref reason }
                if field == "import_data" && reason.contains("duplicate association: a -> b")
        ),
        "unexpected error: {error:?}"
    );

    assert!(framework.get_concept("a").await.unwrap().is_none());
    assert!(framework.get_concept("b").await.unwrap().is_none());
    assert_eq!(
        framework.get_associations("live").await.unwrap(),
        Vec::new()
    );
}

#[tokio::test]
async fn merge_import_resolves_an_endpoint_from_the_live_namespace() {
    let framework = memory_only_framework().await;
    framework
        .inject_concepts(&[("live-target".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let payload = payload(
        vec![export_concept("newcomer")],
        vec![("newcomer".to_string(), "live-target".to_string(), 0.4)],
    );
    let directory = tempfile::tempdir().unwrap();
    let path = write_payload(directory.path(), "merge-live-endpoint.json", &payload);

    assert_eq!(
        framework.import_json(&path, true).await.unwrap(),
        1,
        "a merge import counts the imported concepts"
    );
    assert_eq!(
        framework.get_associations("newcomer").await.unwrap(),
        vec![("live-target".to_string(), 0.4)]
    );
    assert!(
        framework
            .get_concept("live-target")
            .await
            .unwrap()
            .is_some(),
        "merge must keep the concepts already live"
    );
}

#[tokio::test]
async fn backup_and_restore_refuse_a_memory_only_framework() {
    // Both wrappers resolve `self.persistence` before touching the disk, so the
    // contract "no persistence, no backup file" is what the gate can see. Without a
    // lib-target assertion, replacing either body with `Ok(())` reported success.
    let framework = memory_only_framework().await;
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("backup.db");
    let target_str = target.to_str().unwrap();

    let error = framework.backup(target_str).await.unwrap_err();
    assert!(
        matches!(error, MemoryError::UnsupportedOperation(_)),
        "backup without persistence must report UnsupportedOperation, got {error:?}"
    );
    let error = framework.restore(target_str).await.unwrap_err();
    assert!(
        matches!(error, MemoryError::UnsupportedOperation(_)),
        "restore without persistence must report UnsupportedOperation, got {error:?}"
    );
    assert!(
        !target.exists(),
        "a refused backup must not leave a destination file behind"
    );
}
