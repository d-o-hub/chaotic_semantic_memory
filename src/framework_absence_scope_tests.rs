#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::framework::ChaoticSemanticFramework;
use crate::metadata_filter::MetadataFilter;
use crate::retrieval::hybrid::HybridResult;
use csm_traits::AbsenceStore;
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;

#[tokio::test]
async fn filtered_miss_does_not_persist_or_hide_a_match_in_another_filter() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("filtered-absence.db");
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .unwrap();
    let mut metadata = HashMap::new();
    metadata.insert("category".to_string(), json!("allowed"));
    framework
        .inject_text_with_metadata("target", "filteredabsencekeyword", metadata)
        .await
        .unwrap();

    let miss_filter = MetadataFilter::eq("category", json!("excluded"));
    assert!(matches!(
        framework
            .probe_text_filtered("filteredabsencekeyword", 1, &miss_filter)
            .await
            .unwrap(),
        HybridResult::Abstained(_)
    ));

    let persistence = framework.persistence.as_ref().unwrap();
    assert!(persistence.list_absences(0).await.unwrap().is_empty());

    let matching_filter = MetadataFilter::eq("category", json!("allowed"));
    match framework
        .probe_text_filtered("filteredabsencekeyword", 1, &matching_filter)
        .await
        .unwrap()
    {
        HybridResult::Success(results) => assert_eq!(results[0].0, "target"),
        HybridResult::Abstained(_) => panic!("filtered miss suppressed a later matching query"),
    }
}
