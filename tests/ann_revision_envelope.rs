//! ADR-0093: revisioned ANN snapshot validation and durable mutation semantics.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chaotic_semantic_memory::index_envelope::IndexSnapshotEnvelope;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn stale_snapshot_after_inject_is_rejected_on_reload() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();

    // Session 1: inject + persist index envelope at revision R.
    {
        let fw = FrameworkBuilder::new()
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();
        let mut v = HVec10240::zero();
        v.set_bit(1);
        fw.inject_concept("c1", v).await.unwrap();
        fw.persist().await.unwrap();
    }

    // Session 2: inject without re-persisting index → revision advances, envelope stale.
    {
        let fw = FrameworkBuilder::new()
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();
        let mut v = HVec10240::zero();
        v.set_bit(2);
        fw.inject_concept("c2", v).await.unwrap();
        // Intentionally do not call persist() — ANN snapshot still has old revision.
    }

    // Session 3: load_replace must rebuild and surface both concepts.
    {
        let fw = FrameworkBuilder::new()
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();
        let mut q = HVec10240::zero();
        q.set_bit(1);
        let r1 = fw.probe(q, 5).await.unwrap();
        assert!(
            r1.iter().any(|(id, _)| id == "c1"),
            "c1 must remain findable after rebuild"
        );
        let mut q2 = HVec10240::zero();
        q2.set_bit(2);
        let r2 = fw.probe(q2, 5).await.unwrap();
        assert!(
            r2.iter().any(|(id, _)| id == "c2"),
            "c2 must be present after stale snapshot rejection"
        );
    }
}

#[tokio::test]
async fn backend_mismatch_rejects_snapshot() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();
    let p = Persistence::new_local(db_path).await.unwrap();

    // Store envelope claiming wrong backend fingerprint.
    let env = IndexSnapshotEnvelope::new(0, "hnsw:m=16:efc=200:efs=50", vec![1, 2, 3]);
    p.save_index_envelope("_default", "main", &env)
        .await
        .unwrap();

    let fw = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();
    // build() calls load_replace — must not panic; brute-force rebuild.
    let _ = fw.probe(HVec10240::zero(), 1).await.unwrap();
}

#[tokio::test]
async fn corrupt_envelope_does_not_brick_load() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();
    {
        let fw = FrameworkBuilder::new()
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();
        let mut v = HVec10240::zero();
        v.set_bit(5);
        fw.inject_concept("keep-me", v).await.unwrap();
    }
    // Write a magic-prefixed but corrupt envelope blob.
    let p = Persistence::new_local(db_path).await.unwrap();
    let mut bad = b"CSMIDX01".to_vec();
    bad.extend_from_slice(&[0xff; 32]);
    p.save_index("_default", "main", &bad).await.unwrap();

    let fw = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();
    let mut q = HVec10240::zero();
    q.set_bit(5);
    let results = fw.probe(q, 1).await.unwrap();
    assert_eq!(results[0].0, "keep-me");
}

#[tokio::test]
async fn load_merge_rebuilds_union_not_snapshot() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();

    {
        let fw = FrameworkBuilder::new()
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();
        let mut v = HVec10240::zero();
        v.set_bit(10);
        fw.inject_concept("persisted", v).await.unwrap();
        fw.persist().await.unwrap();
    }

    // Start from empty memory + DB rows; inject a second concept in-process,
    // then load_merge again (idempotent) and confirm both remain probeable.
    let fw2 = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();
    let mut v2 = HVec10240::zero();
    v2.set_bit(12);
    fw2.inject_concept("extra", v2).await.unwrap();
    fw2.load_merge().await.unwrap();

    let mut q = HVec10240::zero();
    q.set_bit(10);
    let results = fw2.probe(q, 5).await.unwrap();
    assert!(results.iter().any(|(id, _)| id == "persisted"));
    let mut q2 = HVec10240::zero();
    q2.set_bit(12);
    let results2 = fw2.probe(q2, 5).await.unwrap();
    assert!(results2.iter().any(|(id, _)| id == "extra"));
}
