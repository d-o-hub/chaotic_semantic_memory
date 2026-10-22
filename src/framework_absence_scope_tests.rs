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

/// Two probe families that miss for different reasons must not accumulate
/// attempts into one record: the semantic-only miss says nothing about the
/// keyword path, and reaching `ABSENCE_MIN_ATTEMPTS` there must not let it
/// short-circuit a hybrid probe that could still match on tokens.
#[tokio::test]
async fn abstentions_from_different_probe_modes_keep_separate_records() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("mode-absence.db");
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .unwrap();

    let query = "modeisolationkeyword";
    assert!(matches!(
        framework.probe_text(query, 1).await.unwrap(),
        HybridResult::Abstained(_)
    ));
    for _ in 0..2 {
        assert!(matches!(
            framework.probe_text_semantic(query, 1).await.unwrap(),
            HybridResult::Abstained(_)
        ));
    }

    let records = framework
        .persistence
        .as_ref()
        .unwrap()
        .list_absences(0)
        .await
        .unwrap();
    let ours: Vec<u32> = records
        .iter()
        .filter(|entry| entry.query == query)
        .map(|entry| entry.attempt_count)
        .collect();
    assert_eq!(
        ours.len(),
        2,
        "each probe mode must own its absence record: {records:?}"
    );
    let mut counts = ours;
    counts.sort_unstable();
    assert_eq!(counts, vec![1, 2], "records must not share attempt counts");
}

/// Both halves of the absence contract, observable through the store: an
/// abstaining probe records the absence against the revision it observed,
/// and a successful retrieval afterwards clears that record.
#[tokio::test]
async fn abstentions_are_recorded_and_cleared() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("absence.db");
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .unwrap();
    let store = framework.persistence.as_ref().expect("persistence enabled");
    let query = "content-that-does-not-exist-yet";

    assert!(
        matches!(
            framework.probe_text(query, 5).await.unwrap(),
            HybridResult::Abstained(_)
        ),
        "an empty store must abstain"
    );
    // Records are keyed by retrieval scope, so the assertion finds its own
    // record by query instead of recomputing the scope hash here.
    let recorded = store
        .list_absences(0)
        .await
        .unwrap()
        .into_iter()
        .find(|entry| entry.query == query)
        .expect("an abstention must persist an absence record");
    assert_eq!(
        recorded.namespace,
        framework.namespace().await,
        "the record must be scoped to the namespace it was observed in"
    );
    assert_eq!(
        recorded.namespace_revision, 0,
        "the record must carry the revision the probe observed"
    );
    assert_eq!(recorded.attempt_count, 1);

    // Content the query matches: the next probe retrieves successfully and
    // must clear the record instead of leaving it to suppress the query.
    framework.inject_text("answer", query).await.unwrap();
    assert!(
        matches!(
            framework.probe_text(query, 5).await.unwrap(),
            HybridResult::Success(_)
        ),
        "a matching concept must be retrievable"
    );
    assert!(
        store
            .list_absences(0)
            .await
            .unwrap()
            .into_iter()
            .all(|entry| entry.query != query),
        "a successful retrieval must clear the absence record"
    );
}
