use chaotic_semantic_memory::graph_traversal::TraversalConfig;
use chaotic_semantic_memory::metadata_filter::MetadataFilter;
use chaotic_semantic_memory::prelude::*;
use serde_json::json;
use std::collections::HashMap;

const NS: &str = "_default";

#[tokio::test]
async fn framework_probe_filtered_traverse_and_shortest_path_work() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept_with_metadata(
            "a",
            HVec10240::random(),
            HashMap::from([(String::from("tenant"), json!("alpha"))]),
        )
        .await
        .unwrap();
    framework
        .inject_concept_with_metadata(
            "b",
            HVec10240::random(),
            HashMap::from([(String::from("tenant"), json!("alpha"))]),
        )
        .await
        .unwrap();
    framework
        .inject_concept_with_metadata(
            "c",
            HVec10240::random(),
            HashMap::from([(String::from("tenant"), json!("beta"))]),
        )
        .await
        .unwrap();

    framework.associate("a", "b", 0.9).await.unwrap();
    framework.associate("b", "c", 0.8).await.unwrap();

    let filtered = framework
        .probe_filtered(
            &HVec10240::random(),
            10,
            &MetadataFilter::eq("tenant", "alpha"),
        )
        .await
        .unwrap();
    assert!(filtered.iter().all(|(id, _)| id == "a" || id == "b"));

    let traversal = framework
        .traverse(
            "a",
            TraversalConfig {
                max_depth: 3,
                min_strength: 0.1,
                max_results: 20,
            },
        )
        .await
        .unwrap();
    assert!(traversal.iter().any(|(id, _)| id == "b"));

    let path = framework.shortest_path("a", "c").await.unwrap().unwrap();
    assert_eq!(
        path,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[tokio::test]
async fn framework_subscribe_emits_events() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let mut rx = framework.subscribe();
    framework
        .inject_concept("evt", HVec10240::random())
        .await
        .unwrap();

    let event = rx.recv().await.unwrap();
    match event {
        MemoryEvent::ConceptInjected { id, .. } => assert_eq!(id, "evt"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn builder_with_version_retention_limits_saved_versions() {
    let db_path = format!("/tmp/csm_version_retention_{}.db", std::process::id());
    let _ = tokio::fs::remove_file(&db_path).await;

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.clone())
        .with_version_retention(1)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("v", HVec10240::random())
        .await
        .unwrap();
    framework
        .update_concept_metadata("v", HashMap::from([(String::from("rev"), json!(1))]))
        .await
        .unwrap();
    framework
        .update_concept_metadata("v", HashMap::from([(String::from("rev"), json!(2))]))
        .await
        .unwrap();

    let db = libsql::Builder::new_local(&db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM csm_versions WHERE concept_id = ?1",
            libsql::params!["v"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let count: i64 = row.get(0).unwrap();
    assert_eq!(count, 1);

    drop(framework);
    let _ = tokio::fs::remove_file(&db_path).await;
}
