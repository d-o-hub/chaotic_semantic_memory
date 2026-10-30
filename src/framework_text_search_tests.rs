#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use crate::metadata_filter::MetadataFilter;
use crate::retrieval::hybrid::{HybridMode, HybridResult};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

async fn framework() -> ChaoticSemanticFramework {
    ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap()
}

async fn keyword_ids(
    framework: &ChaoticSemanticFramework,
    query: &str,
    top_k: usize,
) -> Vec<String> {
    match framework
        .probe_text_with_mode(query, top_k, HybridMode::KeywordOnly)
        .await
        .unwrap()
    {
        HybridResult::Success(results) => results.into_iter().map(|(id, _)| id).collect(),
        HybridResult::Abstained(_) => Vec::new(),
    }
}

#[tokio::test]
async fn long_source_keyword_search_reuses_and_invalidates_postings() {
    let framework = framework().await;
    let source = format!("{}needlequasar", "padding ".repeat(35));
    framework.inject_text("target", &source).await.unwrap();

    assert_eq!(keyword_ids(&framework, "needlequasar", 1).await, ["target"]);
    let first = framework.text_index_for("_default", false).await.unwrap();
    let second = framework.text_index_for("_default", false).await.unwrap();
    assert!(Arc::ptr_eq(&first, &second));

    framework
        .update_concept_metadata(
            "target",
            HashMap::from([("searchable_text".to_string(), json!("newsearchterm"))]),
        )
        .await
        .unwrap();
    let updated = framework.text_index_for("_default", false).await.unwrap();
    assert!(!Arc::ptr_eq(&first, &updated));
    assert!(keyword_ids(&framework, "needlequasar", 1).await.is_empty());
    assert_eq!(
        keyword_ids(&framework, "newsearchterm", 1).await,
        ["target"]
    );

    framework.delete_concept("target").await.unwrap();
    assert!(keyword_ids(&framework, "newsearchterm", 1).await.is_empty());
}

#[tokio::test]
async fn keyword_top_k_filters_expired_and_metadata_mismatches_first() {
    let framework = framework().await;
    let mut wrong_metadata = HashMap::new();
    wrong_metadata.insert("group".to_string(), json!("wrong"));
    framework
        .inject_text_with_metadata("wrong", "filterneedle", wrong_metadata)
        .await
        .unwrap();
    let mut correct_metadata = HashMap::new();
    correct_metadata.insert("group".to_string(), json!("right"));
    framework
        .inject_text_with_metadata(
            "right",
            "filterneedle with several extra words",
            correct_metadata,
        )
        .await
        .unwrap();

    let filter = MetadataFilter::eq("group", json!("right"));
    match framework
        .probe_text_filtered("filterneedle", 1, &filter)
        .await
        .unwrap()
    {
        HybridResult::Success(results) => assert_eq!(results[0].0, "right"),
        HybridResult::Abstained(_) => panic!("metadata-eligible keyword match was lost"),
    }

    framework
        .inject_text_with_ttl("expired", "expiryneedle", 0)
        .await
        .unwrap();
    framework
        .inject_text("active", "expiryneedle extra filler words")
        .await
        .unwrap();
    assert_eq!(keyword_ids(&framework, "expiryneedle", 1).await, ["active"]);
}

#[tokio::test]
async fn custom_hybrid_mode_validates_the_keyword_weight() {
    // The match guard at framework_text_search.rs:149 is the only thing between a
    // caller and a nonsense blend: replacing it with `true` accepted weight 2.0,
    // replacing it with `false` rejected weight 0.5. Both directions are pinned here.
    let framework = framework().await;
    framework
        .inject_text("target", "guardneedle extra filler words")
        .await
        .unwrap();

    for valid in [0.0, 0.5, 1.0] {
        let outcome = framework
            .probe_text_with_mode("guardneedle", 1, HybridMode::Custom(valid))
            .await;
        assert!(
            outcome.is_ok(),
            "keyword weight {valid} is in range and must be accepted"
        );
    }
    for invalid in [-0.1, 1.000_1, 2.0, f32::NAN, f32::INFINITY] {
        let outcome = framework
            .probe_text_with_mode("guardneedle", 1, HybridMode::Custom(invalid))
            .await;
        assert!(
            outcome.is_err(),
            "keyword weight {invalid} is out of range or not finite and must be rejected"
        );
    }
}

#[tokio::test]
async fn keyword_search_keeps_a_concept_that_expires_in_the_future() {
    // is_active() reads `expires_at > now`, so a concept that has not expired yet is
    // searchable. Inverting the comparison hides it while leaving permanent
    // concepts visible, which a permanent-only test cannot see.
    let framework = framework().await;
    framework
        .inject_text("permanent", "sharedneedle filler words here")
        .await
        .unwrap();
    framework
        .inject_text_with_ttl("later", "sharedneedle more filler words", 3600)
        .await
        .unwrap();

    let found = keyword_ids(&framework, "sharedneedle", 3).await;
    assert!(
        found.contains(&"later".to_string()),
        "an expiry one hour away is still active and must stay searchable: {found:?}"
    );
    assert!(
        found.contains(&"permanent".to_string()),
        "a concept without an expiry is always active: {found:?}"
    );
}

#[tokio::test]
async fn invalidating_a_namespace_leaves_other_namespaces_postings_cached() {
    // invalidate_text_index() drops the mutated namespace's entries and keeps every
    // other namespace's. Written as `entry_namespace == namespace` it drops the rest
    // of the corpus instead, which is invisible to a query (a rebuilt index answers
    // the same way) but visible to the cache identity the reuse path promises.
    let framework = framework().await;
    for ns in ["alpha", "beta"] {
        framework.set_namespace(ns).await.unwrap();
        framework
            .inject_text("doc", "crossnsneedle filler words")
            .await
            .unwrap();
    }

    let alpha_before = framework.text_index_for("alpha", false).await.unwrap();
    let beta_before = framework.text_index_for("beta", false).await.unwrap();

    framework.invalidate_text_index("alpha").await;

    let beta_after = framework.text_index_for("beta", false).await.unwrap();
    assert!(
        Arc::ptr_eq(&beta_before, &beta_after),
        "invalidating alpha must not evict beta's postings"
    );
    let alpha_after = framework.text_index_for("alpha", false).await.unwrap();
    assert!(
        !Arc::ptr_eq(&alpha_before, &alpha_after),
        "the invalidated namespace must be rebuilt"
    );
}
