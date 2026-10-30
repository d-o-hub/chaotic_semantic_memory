#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::hyperdim::HVec10240;
use tempfile::tempdir;

#[tokio::test]
async fn test_durable_inject_concept_failure_leaves_memory_unchanged() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("test.db");
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to build framework");

    let count_before = fw.stats().await.expect("stats failed").concept_count;
    assert_eq!(count_before, 0);

    let persistence = fw
        .persistence
        .as_ref()
        .expect("Persistence should be present");
    persistence.set_simulate_failure(true);

    let result = fw
        .inject_concept("failed_concept", HVec10240::random())
        .await;
    assert!(
        result.is_err(),
        "inject_concept should fail when persistence fails"
    );

    let count_after = fw.stats().await.expect("stats failed").concept_count;
    assert_eq!(
        count_after, 0,
        "In-memory concept count must remain unchanged after persistence failure"
    );
    assert!(
        fw.get_concept("failed_concept")
            .await
            .expect("get_concept failed")
            .is_none(),
        "Concept must not exist in memory after persistence failure"
    );
}

#[tokio::test]
async fn test_durable_delete_concept_failure_leaves_memory_unchanged() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("test.db");
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to build framework");

    // Successfully inject one concept first
    let vec = HVec10240::random();
    fw.inject_concept("delete_me", vec)
        .await
        .expect("Initial inject_concept failed");

    let count_before = fw.stats().await.expect("stats failed").concept_count;
    assert_eq!(count_before, 1);

    let persistence = fw
        .persistence
        .as_ref()
        .expect("Persistence should be present");
    persistence.set_simulate_failure(true);

    let result = fw.delete_concept("delete_me").await;
    assert!(
        result.is_err(),
        "delete_concept should fail when persistence fails"
    );

    let count_after = fw.stats().await.expect("stats failed").concept_count;
    assert_eq!(
        count_after, 1,
        "In-memory concept count must remain unchanged after failed delete"
    );
    assert!(
        fw.get_concept("delete_me")
            .await
            .expect("get_concept failed")
            .is_some(),
        "Concept must remain in memory after failed delete"
    );
}

#[tokio::test]
async fn test_durable_inject_concepts_batch_failure_leaves_memory_unchanged() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("test.db");
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to build framework");

    let persistence = fw
        .persistence
        .as_ref()
        .expect("Persistence should be present");
    persistence.set_simulate_failure(true);

    let batch = vec![
        ("batch_1".to_string(), HVec10240::random()),
        ("batch_2".to_string(), HVec10240::random()),
    ];

    let result = fw.inject_concepts(&batch).await;
    assert!(
        result.is_err(),
        "inject_concepts batch should fail when persistence fails"
    );

    let count = fw.stats().await.expect("stats failed").concept_count;
    assert_eq!(
        count, 0,
        "In-memory concept count must remain 0 after batch failure"
    );
}

#[tokio::test]
async fn test_load_all_associations_query_count_regression() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("test_assoc_query_count.db");
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .expect("Failed to build framework");

    let persistence = fw
        .persistence
        .as_ref()
        .expect("Persistence should be present");

    for i in 0..=50 {
        let id = format!("c_{i}");
        fw.inject_concept(&id, HVec10240::random())
            .await
            .expect("inject_concept failed");
    }

    for i in 0..50 {
        let from = format!("c_{i}");
        let to = format!("c_{}", i + 1);
        persistence
            .save_association("_default", &from, &to, 0.7)
            .await
            .expect("save_association failed");
    }

    persistence.reset_query_count();
    let associations = persistence
        .load_all_associations("_default")
        .await
        .expect("load_all_associations failed");

    assert_eq!(associations.len(), 50);
    assert_eq!(
        persistence.query_count(),
        1,
        "load_all_associations must issue exactly 1 database query regardless of association count"
    );
}
#[tokio::test]
async fn failed_persistent_mutations_preserve_memory_and_reload_state() {
    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("mutation-failures.db");
    let db = db_path.to_str().unwrap();
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .expect("Failed to build framework");
    let original_vector = HVec10240::random();
    fw.inject_concept("a", original_vector).await.unwrap();
    fw.inject_concepts(&[
        ("b".to_string(), HVec10240::random()),
        ("c".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    fw.associate("a", "b", 0.7).await.unwrap();
    fw.inject_concept_with_ttl("expired", HVec10240::random(), 0)
        .await
        .unwrap();

    let persistence = fw.persistence.as_ref().unwrap();
    persistence.set_simulate_failure(true);

    assert!(
        fw.update_concept_vector("a", HVec10240::random())
            .await
            .is_err()
    );
    let concept = fw.get_concept("a").await.unwrap().unwrap();
    assert_eq!(concept.vector, original_vector);

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("changed".to_string(), serde_json::json!(true));
    assert!(fw.update_concept_metadata("a", metadata).await.is_err());
    assert!(
        fw.get_concept("a")
            .await
            .unwrap()
            .unwrap()
            .metadata
            .is_empty()
    );

    assert!(
        fw.associate_many(&[("a".to_string(), "c".to_string(), 0.4)])
            .await
            .is_err()
    );
    assert_eq!(
        fw.get_associations("a").await.unwrap(),
        vec![("b".into(), 0.7)]
    );

    assert!(fw.disassociate("a", "b").await.is_err());
    assert_eq!(
        fw.get_associations("a").await.unwrap(),
        vec![("b".into(), 0.7)]
    );
    assert!(fw.clear_associations("a").await.is_err());
    assert_eq!(
        fw.get_associations("a").await.unwrap(),
        vec![("b".into(), 0.7)]
    );
    assert!(fw.reinforce_association("a", "b").await.is_err());
    assert!(fw.prune_decayed_associations(1.0).await.is_err());
    assert_eq!(
        fw.get_associations("a").await.unwrap(),
        vec![("b".into(), 0.7)]
    );
    assert!(fw.purge_expired().await.is_err());
    assert!(fw.get_concept("expired").await.unwrap().is_some());

    persistence.set_simulate_failure(false);
    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    reloaded.load().await.unwrap();
    let durable = reloaded.get_concept("a").await.unwrap().unwrap();
    assert_eq!(durable.vector, original_vector);
    assert!(durable.metadata.is_empty());
    assert_eq!(
        reloaded.get_associations("a").await.unwrap(),
        vec![("b".into(), 0.7)]
    );
    assert!(reloaded.get_concept("expired").await.unwrap().is_some());
}

#[tokio::test]
async fn failed_associate_many_does_not_apply_a_valid_prefix() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("a".to_string(), HVec10240::random()),
        ("b".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();

    let result = fw
        .associate_many(&[
            ("a".to_string(), "b".to_string(), 0.7),
            ("a".to_string(), "missing".to_string(), 0.4),
        ])
        .await;

    assert!(result.is_err());
    assert!(
        fw.get_associations("a").await.unwrap().is_empty(),
        "invalid batch must not leave its valid prefix applied"
    );
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn malformed_replace_import_preserves_memory_and_durable_rows() {
    use crate::export_payload::{ExportConcept, ExportPayload};

    let dir = tempdir().expect("Failed to create tempdir");
    let db_path = dir.path().join("import-failure.db");
    let db = db_path.to_str().unwrap();
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    let existing_vector = HVec10240::random();
    fw.inject_concept("existing", existing_vector)
        .await
        .unwrap();

    let payload = ExportPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: 0,
        concepts: vec![ExportConcept {
            id: String::new(),
            vector: HVec10240::random(),
            metadata: std::collections::HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }],
        associations: Vec::new(),
    };
    let import_path = dir.path().join("invalid.json");
    std::fs::write(&import_path, serde_json::to_vec(&payload).unwrap()).unwrap();

    assert!(
        fw.import_json(import_path.to_str().unwrap(), false)
            .await
            .is_err()
    );
    assert_eq!(
        fw.get_concept("existing").await.unwrap().unwrap().vector,
        existing_vector
    );

    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    reloaded.load().await.unwrap();
    assert_eq!(
        reloaded
            .get_concept("existing")
            .await
            .unwrap()
            .unwrap()
            .vector,
        existing_vector
    );
}

#[tokio::test]
async fn cold_namespace_switch_loads_associations() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("cold-namespace.db");
    let db = db_path.to_str().unwrap();
    let writer = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    writer.set_namespace("cold").await.unwrap();
    writer
        .inject_concepts(&[
            ("a".to_string(), HVec10240::random()),
            ("b".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();
    writer.associate("a", "b", 0.65).await.unwrap();
    drop(writer);

    let reader = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    reader.set_namespace("cold").await.unwrap();
    assert_eq!(
        reader.get_associations("a").await.unwrap(),
        vec![("b".to_string(), 0.65)]
    );
}

#[tokio::test]
async fn invalid_association_import_preserves_replaced_namespace() {
    use crate::export_payload::{ExportConcept, ExportPayload};

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("invalid-import-edge.db");
    let db = db_path.to_str().unwrap();
    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    let old_vector = HVec10240::random();
    fw.inject_concept("existing", old_vector).await.unwrap();

    let payload = ExportPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: 0,
        concepts: vec![ExportConcept {
            id: "new".to_string(),
            vector: HVec10240::random(),
            metadata: std::collections::HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }],
        associations: vec![("new".to_string(), "missing".to_string(), 0.7)],
    };
    let import_path = dir.path().join("invalid-edge.json");
    std::fs::write(&import_path, serde_json::to_vec(&payload).unwrap()).unwrap();

    assert!(
        fw.import_json(import_path.to_str().unwrap(), false)
            .await
            .is_err()
    );
    assert_eq!(
        fw.get_concept("existing").await.unwrap().unwrap().vector,
        old_vector
    );
    assert!(fw.get_concept("new").await.unwrap().is_none());

    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    reloaded.load().await.unwrap();
    assert_eq!(
        reloaded
            .get_concept("existing")
            .await
            .unwrap()
            .unwrap()
            .vector,
        old_vector
    );
    assert!(reloaded.get_concept("new").await.unwrap().is_none());
}

#[path = "framework_persistence_import_tests.rs"]
mod import_tests;
