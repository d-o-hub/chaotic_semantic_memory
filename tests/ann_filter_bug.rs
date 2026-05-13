#![cfg(any(feature = "ann-hnsw", feature = "ann-lsh"))]

use chaotic_semantic_memory::MetadataFilter;
use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn test_hnsw_filter_bug_prevention() {
    let framework = FrameworkBuilder::default()
        .without_persistence()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 100,
            ef_search: 50,
        })
        .build()
        .await
        .unwrap();

    // Inject a concept with specific metadata
    framework
        .inject_concept_with_metadata(
            "match",
            HVec10240::random(),
            serde_json::json!({"tag": "valid"})
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
        )
        .await
        .unwrap();

    // Inject a concept that should NOT match
    framework
        .inject_concept_with_metadata(
            "no-match",
            HVec10240::random(),
            serde_json::json!({"tag": "invalid"})
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
        )
        .await
        .unwrap();

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("tag", "valid");

    // Search with filter
    let results = framework.probe_filtered(&query, 10, &filter).await.unwrap();

    // Should only contain "match", not "no-match"
    assert!(results.iter().any(|(id, _)| id == "match"));
    assert!(!results.iter().any(|(id, _)| id == "no-match"));
}

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn test_hnsw_empty_filter_results() {
    let framework = FrameworkBuilder::default()
        .without_persistence()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 100,
            ef_search: 50,
        })
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("nonexistent", "value");

    // Search with filter that matches nothing
    let results = framework.probe_filtered(&query, 10, &filter).await.unwrap();

    // Should be empty, NOT containing c1
    assert!(
        results.is_empty(),
        "Results should be empty when filter matches nothing, but got {results:?}"
    );
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn test_lsh_filter_bug_prevention() {
    let framework = FrameworkBuilder::default()
        .without_persistence()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 5,
            hash_bits: 16,
        })
        .build()
        .await
        .unwrap();

    framework
        .inject_concept_with_metadata(
            "match",
            HVec10240::random(),
            serde_json::json!({"tag": "valid"})
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
        )
        .await
        .unwrap();

    framework
        .inject_concept_with_metadata(
            "no-match",
            HVec10240::random(),
            serde_json::json!({"tag": "invalid"})
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
        )
        .await
        .unwrap();

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("tag", "valid");

    let results = framework.probe_filtered(&query, 10, &filter).await.unwrap();

    assert!(results.iter().any(|(id, _)| id == "match"));
    assert!(!results.iter().any(|(id, _)| id == "no-match"));
}
