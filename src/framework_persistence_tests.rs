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
