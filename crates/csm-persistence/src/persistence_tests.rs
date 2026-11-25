#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use chrono::Utc;
use csm_traits::{AbsenceEntry, AbsenceStore};
use tempfile::NamedTempFile;

#[tokio::test]
async fn absence_store_crud_and_list() {
    let temp = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp.path().to_str().expect("Invalid path");
    let persistence = Persistence::new_local(path)
        .await
        .expect("Failed to create persistence");

    let ts = Utc::now();
    let entry = AbsenceEntry {
        id: AbsenceEntry::id_for("missing query"),
        query: "missing query".to_string(),
        normalized_query: AbsenceEntry::normalize("missing query"),
        attempt_count: 1,
        last_threshold: 0.5,
        best_score_ever: None,
        first_seen: ts,
        last_seen: ts,
        namespace: "_default".to_string(),
        namespace_revision: 1,
    };

    // Unknown id returns None before any upsert.
    assert!(
        persistence
            .get_absence(&entry.id)
            .await
            .expect("get_absence failed")
            .is_none()
    );

    persistence
        .upsert_absence(&entry)
        .await
        .expect("upsert_absence failed");

    let loaded = persistence
        .get_absence(&entry.id)
        .await
        .expect("get_absence failed")
        .expect("absence should exist after upsert");
    assert_eq!(loaded.id, entry.id);
    assert_eq!(loaded.attempt_count, 1);

    // list_absences returns entries at or above the attempt floor.
    let listed = persistence
        .list_absences(1)
        .await
        .expect("list_absences failed");
    assert_eq!(listed.len(), 1);
    assert!(
        persistence
            .list_absences(2)
            .await
            .expect("list_absences failed")
            .is_empty()
    );
}

#[tokio::test]
async fn restore_preserves_graph_rows_and_invalidates_absence_cache() {
    use csm_core_lib::hyperdim::HVec10240;

    const NS: &str = "_default";
    let database = NamedTempFile::new().expect("create database path");
    let backup = NamedTempFile::new().expect("create backup path");
    let persistence = Persistence::new_local(database.path().to_str().unwrap())
        .await
        .expect("initialize persistence");
    let concept = |id: &str| csm_memory::Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };

    persistence
        .save_concept(NS, &concept("backup-a"))
        .await
        .unwrap();
    persistence
        .save_concept(NS, &concept("backup-b"))
        .await
        .unwrap();
    persistence
        .save_association(NS, "backup-a", "backup-b", 0.75)
        .await
        .unwrap();
    let revision = persistence.get_namespace_revision(NS).await.unwrap();
    let now = Utc::now();
    let absence = AbsenceEntry {
        id: AbsenceEntry::id_for("missing before backup"),
        query: "missing before backup".to_string(),
        normalized_query: AbsenceEntry::normalize("missing before backup"),
        attempt_count: 3,
        last_threshold: 0.5,
        best_score_ever: None,
        first_seen: now,
        last_seen: now,
        namespace: NS.to_string(),
        namespace_revision: revision,
    };
    persistence.upsert_absence(&absence).await.unwrap();
    persistence
        .backup(backup.path().to_str().unwrap())
        .await
        .unwrap();

    persistence
        .save_concept(NS, &concept("after-backup"))
        .await
        .unwrap();
    persistence
        .restore(backup.path().to_str().unwrap())
        .await
        .unwrap();

    let concepts = persistence
        .load_all_concepts::<HVec10240>(NS)
        .await
        .unwrap();
    assert_eq!(
        concepts
            .iter()
            .map(|concept| concept.id.as_str())
            .collect::<std::collections::HashSet<_>>(),
        ["backup-a", "backup-b"].into_iter().collect()
    );
    let associations = persistence.load_all_associations(NS).await.unwrap();
    assert!(
        associations.iter().any(|(from, to, strength, _)| {
            from == "backup-a" && to == "backup-b" && (*strength - 0.75).abs() < f32::EPSILON
        }),
        "restore preserves association endpoints and strength"
    );
    assert!(
        persistence.list_absences(1).await.unwrap().is_empty(),
        "restore clears revision-scoped absence cache entries"
    );
    assert!(persistence.get_namespace_revision(NS).await.unwrap() > 0);
}

#[tokio::test]
async fn remote_backup_refuses_and_names_the_export_import_workaround() {
    let temp = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp.path().to_str().expect("Invalid path");
    let local = Persistence::new_local(path)
        .await
        .expect("Failed to create persistence");

    // The remote shape per ADR-0105: a live handle but no local file path.
    // `backup` guards on `local_path` before any I/O, so this construct
    // exercises the refusal contract without a network endpoint.
    let remote = Persistence {
        db: local.db.clone(),
        local_path: None,
        remote_limit: local.remote_limit.clone(),
        version_retention: local.version_retention,
        simulate_failure: AtomicBool::new(false),
        query_count: AtomicU64::new(0),
    };

    let error = remote
        .backup("ignored.bk")
        .await
        .expect_err("remote-shaped backup must refuse");
    let MemoryError::UnsupportedOperation(message) = error else {
        panic!("backup refusal must be UnsupportedOperation, got {error:?}");
    };
    assert!(
        message.contains("only supported for local SQLite databases"),
        "refusal must state the constraint, got: {message}"
    );
    assert!(
        message.contains("export/import round-trip"),
        "refusal must name the export/import workaround (ADR-0105), got: {message}"
    );
}

#[tokio::test]
async fn remote_restore_refuses_and_names_the_export_import_workaround() {
    let temp = NamedTempFile::new().expect("Failed to create temp file");
    let path = temp.path().to_str().expect("Invalid path");
    let local = Persistence::new_local(path)
        .await
        .expect("Failed to create persistence");

    let remote = Persistence {
        db: local.db.clone(),
        local_path: None,
        remote_limit: local.remote_limit.clone(),
        version_retention: local.version_retention,
        simulate_failure: AtomicBool::new(false),
        query_count: AtomicU64::new(0),
    };

    // A snapshot file exists on disk: the refusal must still come from the
    // local-only guard, not from a missing-file IO error.
    let snapshot = NamedTempFile::new().expect("Failed to create snapshot file");
    let error = remote
        .restore(snapshot.path().to_str().expect("Invalid path"))
        .await
        .expect_err("remote-shaped restore must refuse");
    let MemoryError::UnsupportedOperation(message) = error else {
        panic!("restore refusal must be UnsupportedOperation, got {error:?}");
    };
    assert!(
        message.contains("only supported for local SQLite databases"),
        "refusal must state the constraint, got: {message}"
    );
    assert!(
        message.contains("export/import round-trip"),
        "refusal must name the export/import workaround (ADR-0105), got: {message}"
    );
}
