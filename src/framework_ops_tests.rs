#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use csm_core::hyperdim::HVec10240;

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
