#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use crate::metadata_filter::MetadataFilter;
use csm_core_lib::hyperdim::HVec10240;

#[tokio::test]
async fn expired_top_match_does_not_consume_exact_or_filtered_top_k() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let query = HVec10240::zero();
    framework
        .inject_concept_with_ttl("expired", query, 0)
        .await
        .unwrap();
    framework
        .inject_concept("active", HVec10240::random())
        .await
        .unwrap();

    assert_eq!(
        framework.probe(query, 1).await.unwrap()[0].0,
        "active",
        "an expired exact match must not occupy the only result slot"
    );
    let filter = MetadataFilter::Not(Box::new(MetadataFilter::exists("excluded")));
    assert_eq!(
        framework.probe_filtered(&query, 1, &filter).await.unwrap()[0].0,
        "active",
        "expiry filtering must run before metadata-filtered top-k selection"
    );
}

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
#[tokio::test]
async fn durable_purge_removes_expired_concepts_and_edges_before_reload() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("purge.db");
    let db = db_path.to_str().unwrap();
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    framework
        .inject_concept_with_ttl("expired", HVec10240::zero(), 0)
        .await
        .unwrap();
    framework
        .inject_concept("active", HVec10240::random())
        .await
        .unwrap();
    framework.associate("expired", "active", 0.8).await.unwrap();

    assert_eq!(framework.purge_expired().await.unwrap(), 1);
    assert!(framework.get_concept("expired").await.unwrap().is_none());
    drop(framework);

    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db)
        .build()
        .await
        .unwrap();
    assert!(reloaded.get_concept("expired").await.unwrap().is_none());
    assert!(reloaded.get_concept("active").await.unwrap().is_some());
    assert!(
        reloaded
            .get_associations("expired")
            .await
            .unwrap()
            .is_empty()
    );
}
