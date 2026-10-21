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
