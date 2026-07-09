#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::collections::HashMap;

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;
use libsql::Builder;
use tempfile::NamedTempFile;

const NS: &str = "_default";

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
        canonical_concept_ids: Vec::new(),
    };

    persistence.save_concept(NS, &concept).await.unwrap();
    persistence
        .save_association(NS, "alpha", "alpha", 0.5)
        .await
        .unwrap();

    let loaded = persistence.load_concept(NS, "alpha").await.unwrap();
    assert!(loaded.is_some());

    let associations = persistence.load_associations(NS, "alpha").await.unwrap();
    assert_eq!(associations.len(), 1);

    persistence.delete_concept(NS, "alpha").await.unwrap();
    let missing = persistence.load_concept(NS, "alpha").await.unwrap();
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
        canonical_concept_ids: Vec::new(),
    };
    persistence.save_concept(NS, &concept).await.unwrap();

    let result = persistence
        .save_association(NS, "alpha", "missing", 0.5)
        .await;
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
async fn save_and_load_concept_preserves_ttl_and_canonical_concept_ids() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = Concept {
        id: "alpha-ttl".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 2,
        expires_at: Some(777),
        canonical_concept_ids: vec!["concept.anchor".to_string()],
    };

    persistence.save_concept(NS, &concept).await.unwrap();

    let loaded = persistence
        .load_concept(NS, "alpha-ttl")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.expires_at, Some(777));
    assert_eq!(
        loaded.canonical_concept_ids,
        vec!["concept.anchor".to_string()]
    );
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
        canonical_concept_ids: Vec::new(),
    };
    persistence.save_concept(NS, &concept_alpha).await.unwrap();
    persistence.backup(backup_path).await.unwrap();

    let concept_beta = Concept {
        id: "beta".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 2,
        modified_at: 2,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };
    persistence.save_concept(NS, &concept_beta).await.unwrap();

    persistence.restore(backup_path).await.unwrap();
    let alpha = persistence.load_concept(NS, "alpha").await.unwrap();
    let beta = persistence.load_concept(NS, "beta").await.unwrap();
    assert!(alpha.is_some());
    assert!(beta.is_none());
}

#[tokio::test]
async fn backup_and_restore_preserves_ttl_and_canonical_concept_ids() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path().to_str().unwrap();
    let backup = NamedTempFile::new().unwrap();
    let backup_path = backup.path().to_str().unwrap();

    let persistence = Persistence::new_local(db_path).await.unwrap();
    let concept = Concept {
        id: "semantic-bridge-anchor".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 10,
        modified_at: 11,
        expires_at: Some(123_456),
        canonical_concept_ids: vec!["concept.alpha".to_string(), "concept.beta".to_string()],
    };

    persistence.save_concept(NS, &concept).await.unwrap();
    persistence.backup(backup_path).await.unwrap();

    // Mutate live DB after backup so restore must recover original fields.
    let replacement = Concept {
        id: "semantic-bridge-anchor".to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 10,
        modified_at: 12,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };
    persistence.save_concept(NS, &replacement).await.unwrap();

    persistence.restore(backup_path).await.unwrap();
    let restored = persistence
        .load_concept(NS, "semantic-bridge-anchor")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(restored.expires_at, Some(123_456));
    assert_eq!(
        restored.canonical_concept_ids,
        vec!["concept.alpha".to_string(), "concept.beta".to_string()]
    );
}

#[tokio::test]
async fn v5_namespace_migration_handles_legacy_and_prefixed_tables() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path().to_str().unwrap();
    let persistence = Persistence::new_local(db_path).await.unwrap();

    let conn = Builder::new_local(db_path)
        .build()
        .await
        .unwrap()
        .connect()
        .unwrap();

    let vector = HVec10240::random().to_bytes();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS concepts (
             id TEXT PRIMARY KEY,
             vector BLOB NOT NULL,
             metadata TEXT NOT NULL,
             created_at INTEGER NOT NULL,
             modified_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS associations (
             from_id TEXT NOT NULL,
             to_id TEXT NOT NULL,
             strength REAL NOT NULL,
             PRIMARY KEY (from_id, to_id)
         );
         CREATE TABLE IF NOT EXISTS concept_versions (
             concept_id TEXT NOT NULL,
             version INTEGER NOT NULL,
             vector BLOB NOT NULL,
             metadata TEXT NOT NULL,
             modified_at INTEGER NOT NULL,
             PRIMARY KEY (concept_id, version)
         );
         CREATE TABLE IF NOT EXISTS canonical_concepts (
             id TEXT PRIMARY KEY,
             version INTEGER NOT NULL,
             labels_json TEXT NOT NULL,
             related_json TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS __schema_version (
             version INTEGER PRIMARY KEY
         );",
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO concepts (id, vector, metadata, created_at, modified_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        libsql::params!["legacy-alpha", vector, "{}", 1_i64, 1_i64],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT OR REPLACE INTO associations (from_id, to_id, strength)
         VALUES (?1, ?2, ?3)",
        libsql::params!["legacy-alpha", "legacy-alpha", 0.9_f64],
    )
    .await
    .unwrap();

    conn.execute_batch(
        "DELETE FROM csm_schema_version;
         INSERT INTO csm_schema_version(version) VALUES (4);",
    )
    .await
    .unwrap();

    persistence.apply_migrations(6).await.unwrap();

    let loaded = persistence.load_concept(NS, "legacy-alpha").await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(persistence.schema_version().await.unwrap(), 6);

    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='concepts'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let legacy_table_count: i64 = row.get(0).unwrap();
    assert_eq!(legacy_table_count, 0);
}
