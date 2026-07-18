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
