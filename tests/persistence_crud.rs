use std::collections::HashMap;

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use libsql::Builder;
use tempfile::NamedTempFile;

const NS: &str = "_default";

fn make_concept(id: &str, created_at: u64, modified_at: u64) -> Concept {
    Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at,
        modified_at,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

fn make_concept_with_meta(id: &str, meta_key: &str, meta_value: &str) -> Concept {
    let mut metadata = HashMap::new();
    metadata.insert(meta_key.to_string(), serde_json::json!(meta_value));
    Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata,
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

async fn sqlite_journal_mode(path: &str) -> String {
    let db = Builder::new_local(path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let mut rows = conn.query("PRAGMA journal_mode;", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let mode: String = row.get(0).unwrap();
    mode.to_ascii_lowercase()
}

#[tokio::test]
async fn concept_lifecycle_save_load_delete() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = make_concept("test-concept", 100, 200);
    let original_vector = concept.vector;

    persistence.save_concept(NS, &concept).await.unwrap();

    let loaded = persistence.load_concept(NS, "test-concept").await.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, "test-concept");
    assert_eq!(loaded.created_at, 100);
    assert_eq!(loaded.modified_at, 200);
    assert!((loaded.vector.cosine_similarity(&original_vector) - 1.0).abs() < 0.001);

    persistence
        .delete_concept(NS, "test-concept")
        .await
        .unwrap();

    let missing = persistence.load_concept(NS, "test-concept").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn concept_update_replaces_existing() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept_v1 = make_concept("updatable", 1, 1);
    persistence.save_concept(NS, &concept_v1).await.unwrap();

    let concept_v2 = make_concept_with_meta("updatable", "version", "2");
    persistence.save_concept(NS, &concept_v2).await.unwrap();

    let loaded = persistence
        .load_concept(NS, "updatable")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.metadata.get("version").unwrap(), "2");
    assert_eq!(loaded.created_at, 1);
}

#[tokio::test]
async fn load_nonexistent_concept_returns_none() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let result = persistence
        .load_concept(NS, "does-not-exist")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn batch_save_concepts_saves_all() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concepts = vec![
        make_concept("batch-1", 1, 1),
        make_concept("batch-2", 2, 2),
        make_concept("batch-3", 3, 3),
    ];

    persistence.save_concepts(NS, &concepts).await.unwrap();

    for (i, id) in ["batch-1", "batch-2", "batch-3"].iter().enumerate() {
        let loaded = persistence.load_concept(NS, id).await.unwrap().unwrap();
        assert_eq!(loaded.created_at, (i + 1) as u64);
    }

    let all = persistence.load_all_concepts(NS).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn batch_save_concepts_preserves_ttl_and_canonical_ids() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = Concept {
        id: "batch-ttl-canonical".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 10,
        modified_at: 11,
        expires_at: Some(123_456),
        canonical_concept_ids: vec!["concept.alpha".to_string(), "concept.beta".to_string()],
    };

    persistence.save_concepts(NS, &[concept]).await.unwrap();

    let loaded = persistence
        .load_concept(NS, "batch-ttl-canonical")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.expires_at, Some(123_456));
    assert_eq!(
        loaded.canonical_concept_ids,
        vec!["concept.alpha".to_string(), "concept.beta".to_string()]
    );
}

#[tokio::test]
async fn batch_save_empty_vec_is_noop() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let result = persistence.save_concepts(NS, &[]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn association_lifecycle_save_load() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("from-id", 1, 1))
        .await
        .unwrap();
    persistence
        .save_concept(NS, &make_concept("to-id", 1, 1))
        .await
        .unwrap();

    persistence
        .save_association(NS, "from-id", "to-id", 0.75)
        .await
        .unwrap();

    let associations = persistence.load_associations(NS, "from-id").await.unwrap();
    assert_eq!(associations.len(), 1);
    assert_eq!(associations[0].0, "to-id");
    assert!((associations[0].1 - 0.75).abs() < 0.001);

    persistence
        .save_association(NS, "from-id", "to-id", 0.5)
        .await
        .unwrap();
    let updated = persistence.load_associations(NS, "from-id").await.unwrap();
    assert_eq!(updated.len(), 1);
    assert!((updated[0].1 - 0.5).abs() < 0.001);
}

#[tokio::test]
async fn association_rejected_for_missing_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("exists", 1, 1))
        .await
        .unwrap();

    let result = persistence
        .save_association(NS, "exists", "missing", 0.5)
        .await;
    assert!(result.is_err());

    let result = persistence
        .save_association(NS, "missing", "exists", 0.5)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn load_associations_empty_for_unknown_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let associations = persistence.load_associations(NS, "unknown").await.unwrap();
    assert!(associations.is_empty());
}

#[tokio::test]
async fn batch_save_associations() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("a", 1, 1))
        .await
        .unwrap();
    persistence
        .save_concept(NS, &make_concept("b", 1, 1))
        .await
        .unwrap();
    persistence
        .save_concept(NS, &make_concept("c", 1, 1))
        .await
        .unwrap();

    let associations = vec![
        ("a".to_string(), "b".to_string(), 0.3f32),
        ("a".to_string(), "c".to_string(), 0.7f32),
        ("b".to_string(), "c".to_string(), 0.5f32),
    ];

    persistence
        .save_associations(NS, &associations)
        .await
        .unwrap();

    let from_a = persistence.load_associations(NS, "a").await.unwrap();
    assert_eq!(from_a.len(), 2);

    let from_b = persistence.load_associations(NS, "b").await.unwrap();
    assert_eq!(from_b.len(), 1);
    assert_eq!(from_b[0].0, "c");
}

#[tokio::test]
async fn batch_save_associations_empty_is_noop() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let result = persistence.save_associations(NS, &[]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn cascade_delete_removes_associations() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("target", 1, 1))
        .await
        .unwrap();
    persistence
        .save_concept(NS, &make_concept("other", 1, 1))
        .await
        .unwrap();
    persistence
        .save_concept(NS, &make_concept("third", 1, 1))
        .await
        .unwrap();

    persistence
        .save_association(NS, "target", "other", 0.5)
        .await
        .unwrap();
    persistence
        .save_association(NS, "other", "target", 0.5)
        .await
        .unwrap();
    persistence
        .save_association(NS, "target", "third", 0.3)
        .await
        .unwrap();

    persistence.delete_concept(NS, "target").await.unwrap();

    let from_target = persistence.load_associations(NS, "target").await.unwrap();
    assert!(from_target.is_empty());

    let from_other = persistence.load_associations(NS, "other").await.unwrap();
    assert!(from_other.is_empty());

    let from_third = persistence.load_associations(NS, "third").await.unwrap();
    assert!(from_third.is_empty());
}

#[tokio::test]
async fn clear_all_removes_everything() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concepts(
            NS,
            &[make_concept("clear-1", 1, 1), make_concept("clear-2", 2, 2)],
        )
        .await
        .unwrap();

    persistence
        .save_association(NS, "clear-1", "clear-2", 0.5)
        .await
        .unwrap();

    persistence.clear_all().await.unwrap();

    let all_concepts = persistence.load_all_concepts(NS).await.unwrap();
    assert!(all_concepts.is_empty());

    let associations = persistence.load_associations(NS, "clear-1").await.unwrap();
    assert!(associations.is_empty());
}

#[tokio::test]
async fn concept_history_tracks_versions() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let v1 = make_concept("history-test", 1, 1);
    persistence.save_concept(NS, &v1).await.unwrap();

    let v2 = make_concept_with_meta("history-test", "v", "2");
    persistence.save_concept(NS, &v2).await.unwrap();

    let v3 = make_concept_with_meta("history-test", "v", "3");
    persistence.save_concept(NS, &v3).await.unwrap();

    let history = persistence
        .get_concept_history(NS, "history-test", 10)
        .await
        .unwrap();
    assert!(!history.is_empty(), "Version history should not be empty");

    let latest = &history[0];
    assert_eq!(latest.concept_id, "history-test");
    assert!(latest.version >= 1);
    assert!(latest.vector.is_some());
    assert!(latest.metadata.is_some());
}

#[tokio::test]
async fn concept_history_respects_limit() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    for i in 0..5 {
        let concept = make_concept("limited-history", 1, i);
        persistence.save_concept(NS, &concept).await.unwrap();
    }

    let history = persistence
        .get_concept_history(NS, "limited-history", 2)
        .await
        .unwrap();
    assert!(!history.is_empty());
    assert!(history.len() <= 2, "History should respect limit");
}

#[tokio::test]
async fn concept_history_empty_for_unknown() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let history = persistence
        .get_concept_history(NS, "unknown", 10)
        .await
        .unwrap();
    assert!(history.is_empty());
}

#[tokio::test]
async fn checkpoint_succeeds() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("checkpoint-test", 1, 1))
        .await
        .unwrap();

    persistence.checkpoint().await.unwrap();
    let mode = sqlite_journal_mode(path).await;
    assert_eq!(mode, "wal");
}

#[tokio::test]
async fn local_sqlite_enables_wal_mode() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let _persistence = Persistence::new_local(path).await.unwrap();

    let mode = sqlite_journal_mode(path).await;
    assert_eq!(mode, "wal");
}

#[tokio::test]
async fn metadata_preserved_across_roundtrip() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("string".to_string(), serde_json::json!("value"));
    metadata.insert("number".to_string(), serde_json::json!(42));
    metadata.insert("nested".to_string(), serde_json::json!({"key": "val"}));

    let concept = Concept {
        id: "meta-test".to_string(),
        vector: HVec10240::random(),
        metadata,
        created_at: 100,
        modified_at: 200,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };

    persistence.save_concept(NS, &concept).await.unwrap();

    let loaded = persistence
        .load_concept(NS, "meta-test")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.metadata.get("string").unwrap(), "value");
    assert_eq!(loaded.metadata.get("number").unwrap(), 42);
    assert_eq!(loaded.metadata.get("nested").unwrap()["key"], "val");
}

#[tokio::test]
async fn vector_integrity_preserved() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let original = HVec10240::random();
    let concept = Concept {
        id: "vector-test".to_string(),
        vector: original,
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };

    persistence.save_concept(NS, &concept).await.unwrap();

    let loaded = persistence
        .load_concept(NS, "vector-test")
        .await
        .unwrap()
        .unwrap();
    assert!((loaded.vector.cosine_similarity(&original) - 1.0).abs() < 0.001);
}

#[tokio::test]
async fn concurrent_reads_are_safe() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("concurrent-read", 1, 1))
        .await
        .unwrap();

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let path = path.to_string();
            tokio::spawn(async move {
                let p = Persistence::new_local(&path).await.unwrap();
                for _ in 0..5 {
                    let result = p.load_concept(NS, "concurrent-read").await.unwrap();
                    assert!(result.is_some());
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn concurrent_writes_are_safe() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let path = path.to_string();
            tokio::spawn(async move {
                let p = Persistence::new_local(&path).await.unwrap();
                for j in 0..5 {
                    let concept = make_concept(&format!("concurrent-{i}-{j}"), i as u64, j as u64);
                    p.save_concept(NS, &concept).await.unwrap();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    let all = persistence.load_all_concepts(NS).await.unwrap();
    assert_eq!(all.len(), 25);
}

#[tokio::test]
async fn multiple_associations_for_single_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concepts(
            NS,
            &[
                make_concept("hub", 1, 1),
                make_concept("spoke-1", 1, 1),
                make_concept("spoke-2", 1, 1),
                make_concept("spoke-3", 1, 1),
            ],
        )
        .await
        .unwrap();

    persistence
        .save_associations(
            NS,
            &[
                ("hub".to_string(), "spoke-1".to_string(), 0.9f32),
                ("hub".to_string(), "spoke-2".to_string(), 0.7f32),
                ("hub".to_string(), "spoke-3".to_string(), 0.5f32),
            ],
        )
        .await
        .unwrap();

    let associations = persistence.load_associations(NS, "hub").await.unwrap();
    assert_eq!(associations.len(), 3);

    let ids: std::collections::HashSet<_> =
        associations.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains("spoke-1"));
    assert!(ids.contains("spoke-2"));
    assert!(ids.contains("spoke-3"));
}

#[tokio::test]
async fn self_association_allowed() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    persistence
        .save_concept(NS, &make_concept("self-ref", 1, 1))
        .await
        .unwrap();

    let result = persistence
        .save_association(NS, "self-ref", "self-ref", 1.0)
        .await;
    assert!(result.is_ok());

    let associations = persistence.load_associations(NS, "self-ref").await.unwrap();
    assert_eq!(associations.len(), 1);
    assert_eq!(associations[0].0, "self-ref");
}

#[tokio::test]
async fn delete_nonexistent_concept_succeeds() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let result = persistence.delete_concept(NS, "nonexistent").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn database_size_increases_with_data() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let initial_size = persistence.size().await.unwrap();

    for i in 0..100 {
        persistence
            .save_concept(NS, &make_concept(&format!("size-test-{i}"), 1, 1))
            .await
            .unwrap();
    }

    let final_size = persistence.size().await.unwrap();
    assert!(final_size > initial_size);
}

#[tokio::test]
async fn health_check_succeeds() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let result = persistence.health_check().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn schema_version_is_valid() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let version = persistence.schema_version().await.unwrap();
    assert!(version >= 2);
}

#[tokio::test]
async fn version_history_deleted_with_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = make_concept("version-delete-test", 1, 1);
    persistence.save_concept(NS, &concept).await.unwrap();
    persistence
        .save_concept(NS, &make_concept("version-delete-test", 1, 2))
        .await
        .unwrap();

    let history_before = persistence
        .get_concept_history(NS, "version-delete-test", 10)
        .await
        .unwrap();
    assert!(
        !history_before.is_empty(),
        "Version history should exist before delete"
    );

    persistence
        .delete_concept(NS, "version-delete-test")
        .await
        .unwrap();

    let history_after = persistence
        .get_concept_history(NS, "version-delete-test", 10)
        .await
        .unwrap();
    assert!(
        history_after.is_empty(),
        "Version history should be deleted with concept"
    );
}

#[tokio::test]
async fn batch_save_with_duplicate_ids_updates() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concepts = vec![
        make_concept("dup", 1, 1),
        make_concept_with_meta("dup", "v", "updated"),
    ];

    persistence.save_concepts(NS, &concepts).await.unwrap();

    let loaded = persistence.load_concept(NS, "dup").await.unwrap().unwrap();
    assert_eq!(loaded.metadata.get("v").unwrap(), "updated");
}
