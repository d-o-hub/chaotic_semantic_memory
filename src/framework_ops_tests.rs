#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::error::MemoryError;
use csm_core_lib::hyperdim::HVec10240;

#[tokio::test]
async fn probe_batch_and_cached_return_injected_concept() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let vec = HVec10240::random();
    fw.inject_concepts(&[("a".to_string(), vec)]).await.unwrap();

    let r1 = fw.probe_batch(&[vec], 1).await.unwrap();
    assert_eq!(r1[0][0].0, "a");

    let r2 = fw.probe_batch_cached(&[vec], 1).await.unwrap();
    assert_eq!(r2[0][0].0, "a");
}

#[tokio::test]
async fn inject_concepts_batch_is_queryable() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let batch: Vec<(String, HVec10240)> = (0..32)
        .map(|i| (format!("c{i}"), HVec10240::random()))
        .collect();
    fw.inject_concepts(&batch).await.unwrap();
    let hits = fw.probe_batch(&[batch[0].1], 1).await.unwrap();
    assert_eq!(hits[0][0].0, "c0");
}

#[tokio::test]
async fn disassociate_and_clear_use_single_namespace_clone() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let a = HVec10240::random();
    let b = HVec10240::random();
    fw.inject_concepts(&[("a".into(), a), ("b".into(), b)])
        .await
        .unwrap();
    fw.associate_many(&[("a".into(), "b".into(), 0.9)])
        .await
        .unwrap();
    fw.disassociate("a", "b").await.unwrap();
    fw.clear_associations("a").await.unwrap();
}

#[tokio::test]
async fn import_json_roundtrip_with_associations() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let a = HVec10240::random();
    let b = HVec10240::random();
    fw.inject_concepts(&[("a".into(), a), ("b".into(), b)])
        .await
        .unwrap();
    fw.associate_many(&[("a".into(), "b".into(), 0.5)])
        .await
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mem.json");
    fw.export_json(path.to_str().unwrap()).await.unwrap();

    let fw2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let n = fw2
        .import_json(path.to_str().unwrap(), false)
        .await
        .unwrap();
    assert_eq!(n, 2);
    let hits = fw2.probe_batch(&[a], 1).await.unwrap();
    assert_eq!(hits[0][0].0, "a");
}

#[tokio::test]
async fn secure_read_file_exact_limit_is_allowed() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("exact.txt");
    std::fs::write(&path, b"hello").unwrap();
    assert!(fw.secure_read_file(path.as_path(), 5).await.is_ok());
}

#[tokio::test]
async fn secure_read_file_over_limit_is_rejected() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("over.txt");
    std::fs::write(&path, b"hello!").unwrap();
    assert!(fw.secure_read_file(path.as_path(), 5).await.is_err());
}

// --- Mutation-killing side-effect assertions (PR #532) ---
// These tests verify *observable* side effects so that mutants replacing a
// function body with `Ok(())` / `Ok(0)` / `Ok(vec![])` are caught by `--lib`.

#[tokio::test]
async fn associate_many_creates_visible_associations() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("src".to_string(), HVec10240::random()),
        ("dst1".to_string(), HVec10240::random()),
        ("dst2".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    fw.associate_many(&[
        ("src".to_string(), "dst1".to_string(), 0.7),
        ("src".to_string(), "dst2".to_string(), 0.3),
    ])
    .await
    .unwrap();
    let links = fw.get_associations("src").await.unwrap();
    let mut by_id: std::collections::HashMap<String, f32> = links.into_iter().collect();
    assert_eq!(by_id.remove("dst1"), Some(0.7));
    assert_eq!(by_id.remove("dst2"), Some(0.3));
    assert!(by_id.is_empty(), "no extra associations: {by_id:?}");
}

#[tokio::test]
async fn update_concept_vector_replaces_stored_vector() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let original = HVec10240::random();
    fw.inject_concepts(&[("vc".to_string(), original)])
        .await
        .unwrap();
    let next = HVec10240::random();
    fw.update_concept_vector("vc", next).await.unwrap();
    let stored = fw.get_concept("vc").await.unwrap().unwrap();
    assert_eq!(stored.vector, next, "vector must be replaced after update");
}

#[tokio::test]
async fn update_concept_metadata_replaces_stored_metadata() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let mut m0 = std::collections::HashMap::new();
    m0.insert("k".to_string(), serde_json::json!("v0"));
    fw.inject_concept_with_metadata("mc", HVec10240::random(), m0)
        .await
        .unwrap();
    let mut m1 = std::collections::HashMap::new();
    m1.insert("k".to_string(), serde_json::json!("v1"));
    m1.insert("added".to_string(), serde_json::json!(true));
    fw.update_concept_metadata("mc", m1).await.unwrap();
    let stored = fw.get_concept("mc").await.unwrap().unwrap();
    assert_eq!(stored.metadata.get("k"), Some(&serde_json::json!("v1")));
    assert_eq!(stored.metadata.get("added"), Some(&serde_json::json!(true)));
    assert_eq!(stored.metadata.len(), 2);
}

#[tokio::test]
async fn disassociate_removes_visible_link() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("d_from".to_string(), HVec10240::random()),
        ("d_to".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    fw.associate_many(&[("d_from".to_string(), "d_to".to_string(), 0.6)])
        .await
        .unwrap();
    assert_eq!(fw.get_associations("d_from").await.unwrap().len(), 1);
    fw.disassociate("d_from", "d_to").await.unwrap();
    let after = fw.get_associations("d_from").await.unwrap();
    assert!(after.is_empty(), "association must be removed: {after:?}");
}

#[tokio::test]
async fn clear_associations_empties_all_outbound() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("hub".to_string(), HVec10240::random()),
        ("spoke1".to_string(), HVec10240::random()),
        ("spoke2".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    fw.associate_many(&[
        ("hub".to_string(), "spoke1".to_string(), 0.5),
        ("hub".to_string(), "spoke2".to_string(), 0.4),
    ])
    .await
    .unwrap();
    assert_eq!(fw.get_associations("hub").await.unwrap().len(), 2);
    fw.clear_associations("hub").await.unwrap();
    let after = fw.get_associations("hub").await.unwrap();
    assert!(after.is_empty(), "all outbound must be cleared: {after:?}");
}

#[tokio::test]
async fn import_binary_roundtrip_returns_correct_count() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("bin-a".to_string(), HVec10240::random()),
        ("bin-b".to_string(), HVec10240::random()),
        ("bin-c".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mem.bin");
    fw.export_binary(path.to_str().unwrap()).await.unwrap();

    let fw2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let n = fw2
        .import_binary(path.to_str().unwrap(), false)
        .await
        .unwrap();
    assert_eq!(n, 3, "import_binary must return the concept count");
    let stats = fw2.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);
}

#[tokio::test]
async fn import_json_replace_mode_clears_preexisting_concepts() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("keep-a".to_string(), HVec10240::random()),
        ("keep-b".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replace.json");
    fw.export_json(path.to_str().unwrap()).await.unwrap();

    // Target starts with an unrelated pre-existing concept.
    let target = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    target
        .inject_concepts(&[("preexisting".to_string(), HVec10240::random())])
        .await
        .unwrap();
    assert_eq!(target.stats().await.unwrap().concept_count, 1);

    let n = target
        .import_json(path.to_str().unwrap(), false)
        .await
        .unwrap();
    assert_eq!(n, 2);
    // merge=false must have cleared the pre-existing concept.
    assert!(target.get_concept("preexisting").await.unwrap().is_none());
    assert!(target.get_concept("keep-a").await.unwrap().is_some());
    assert!(target.get_concept("keep-b").await.unwrap().is_some());
}

#[tokio::test]
async fn import_binary_replace_mode_clears_preexisting_concepts() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[("rb-a".to_string(), HVec10240::random())])
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replace.bin");
    fw.export_binary(path.to_str().unwrap()).await.unwrap();

    let target = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    target
        .inject_concepts(&[("rb-old".to_string(), HVec10240::random())])
        .await
        .unwrap();
    target
        .import_binary(path.to_str().unwrap(), false)
        .await
        .unwrap();
    assert!(
        target.get_concept("rb-old").await.unwrap().is_none(),
        "merge=false must clear pre-existing concepts"
    );
    assert!(target.get_concept("rb-a").await.unwrap().is_some());
}

#[cfg(not(miri))]
#[tokio::test]
async fn import_with_persistence_is_loadable_from_fresh_framework() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("imported.db");
    let db_path = db.to_str().unwrap();

    // Source framework (no persistence) produces a JSON export.
    let src = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    src.inject_concepts(&[
        ("imp-1".to_string(), HVec10240::random()),
        ("imp-2".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    let export_path = dir.path().join("exp.json");
    src.export_json(export_path.to_str().unwrap())
        .await
        .unwrap();

    // Target framework WITH persistence imports the payload.
    let target = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();
    let n = target
        .import_json(export_path.to_str().unwrap(), false)
        .await
        .unwrap();
    assert_eq!(n, 2);
    target.persist().await.unwrap();

    // A fresh framework on the same DB must see the imported concepts.
    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();
    reloaded.load().await.unwrap();
    assert!(reloaded.get_concept("imp-1").await.unwrap().is_some());
    assert!(reloaded.get_concept("imp-2").await.unwrap().is_some());
}

#[cfg(not(miri))]
#[tokio::test]
async fn concept_history_with_persistence_returns_versions_after_update() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("history.db");
    let db_path = db.to_str().unwrap();

    let fw = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .with_version_retention(10)
        .build()
        .await
        .unwrap();
    fw.inject_concept_with_metadata(
        "hist-c",
        HVec10240::random(),
        std::collections::HashMap::new(),
    )
    .await
    .unwrap();
    fw.persist().await.unwrap();

    // Vector update must create a new version record.
    fw.update_concept_vector("hist-c", HVec10240::random())
        .await
        .unwrap();
    fw.persist().await.unwrap();

    let history = fw.concept_history("hist-c", 100).await.unwrap();
    assert!(
        !history.is_empty(),
        "concept_history must return recorded versions, got empty"
    );
}

#[tokio::test]
async fn prune_decayed_associations_strength_validation_fails_unit() {
    let ttl_config = crate::framework_ttl_advanced::TtlConfig {
        association_decay: crate::framework_ttl_advanced::DecayCurve::Step {
            threshold_seconds: 0,
            drop: 1.0,
        },
        ..Default::default()
    };

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_ttl_config(ttl_config)
        .build()
        .await
        .unwrap();

    // Too low threshold
    let result = framework.prune_decayed_associations(-0.1).await;
    assert!(result.is_err());

    // Too high threshold
    let result = framework.prune_decayed_associations(1.1).await;
    assert!(result.is_err());

    // Infinite threshold
    let result = framework.prune_decayed_associations(f32::INFINITY).await;
    assert!(result.is_err());

    // NaN threshold
    let result = framework.prune_decayed_associations(f32::NAN).await;
    assert!(result.is_err());

    // Error must name the `threshold` parameter, not `strength`
    let err = framework
        .prune_decayed_associations(f32::NAN)
        .await
        .unwrap_err();
    let MemoryError::InvalidInput { field, .. } = err else {
        panic!("expected InvalidInput, got: {err:?}");
    };
    assert_eq!(field, "threshold");

    // Inclusive range edges are valid (with no associations, prune returns Ok(0))
    let result = framework.prune_decayed_associations(0.0).await;
    assert!(result.is_ok());
    let result = framework.prune_decayed_associations(1.0).await;
    assert!(result.is_ok());

    // Setup active associations to verify pruning logic and kill cargo-mutants (expect exactly 2 pruned)
    framework
        .inject_concept("assoc-prune-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-prune-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-prune-3", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("assoc-prune-1", "assoc-prune-2", 0.9)
        .await
        .unwrap();
    framework
        .associate("assoc-prune-1", "assoc-prune-3", 0.9)
        .await
        .unwrap();

    // Valid threshold (should be Ok and return exactly 2 pruned associations)
    let result = framework.prune_decayed_associations(0.5).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
}
