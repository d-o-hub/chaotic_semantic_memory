//! Tests for selectivity-aware filtered retrieval (ADR-0065).

use chaotic_semantic_memory::{
    ConceptBuilder, FilterStrategy, HVec10240, MetadataFilter, singularity::Singularity,
};
use serde_json::json;

fn inject_concept(
    sing: &mut Singularity,
    id: &str,
    metadata_key: &str,
    metadata_val: serde_json::Value,
) {
    let vector = HVec10240::random();
    let concept = ConceptBuilder::new(id)
        .with_vector(vector)
        .with_metadata(metadata_key, metadata_val)
        .build()
        .unwrap();
    sing.inject(concept).unwrap();
}

#[test]
fn test_find_similar_filtered_returns_matching_only() {
    let mut sing = Singularity::new();

    // Inject concepts with different categories
    inject_concept(&mut sing, "doc1", "category", json!("document"));
    inject_concept(&mut sing, "doc2", "category", json!("document"));
    inject_concept(&mut sing, "img1", "category", json!("image"));

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("category", "document");

    let results = sing.find_similar_filtered(&query, 10, &filter);

    // All results should be documents - check via get()
    for (id, _) in results.iter() {
        let concept = sing.get(id).unwrap();
        assert_eq!(concept.metadata.get("category"), Some(&json!("document")));
    }
}

#[test]
fn test_find_similar_filtered_empty_result() {
    let mut sing = Singularity::new();

    inject_concept(&mut sing, "doc1", "category", json!("document"));

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("category", "video"); // No videos exist

    let results = sing.find_similar_filtered(&query, 10, &filter);
    assert!(results.is_empty());
}

#[test]
fn test_retrieval_stats_contains_selectivity_ratio() {
    let mut sing = Singularity::new();

    // Create 10 concepts: 3 matching filter (30% selectivity)
    for i in 0..3 {
        inject_concept(&mut sing, &format!("match{}", i), "type", json!("match"));
    }
    for i in 0..7 {
        inject_concept(&mut sing, &format!("other{}", i), "type", json!("other"));
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("type", "match");

    let _ = sing.find_similar_filtered(&query, 5, &filter);

    let stats = sing.last_retrieval_stats();
    // 3 matching out of 10 total = 0.3 selectivity
    assert!((stats.selectivity_ratio - 0.3).abs() < 0.01);
}

#[test]
fn test_small_dataset_uses_pre_filter() {
    let mut sing = Singularity::new();

    // Only 5 concepts (< 20 threshold)
    for i in 0..5 {
        inject_concept(&mut sing, &format!("c{}", i), "tag", json!(i % 2));
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("tag", 1);

    let _ = sing.find_similar_filtered(&query, 3, &filter);

    let stats = sing.last_retrieval_stats();
    // Small dataset should always use Pre strategy regardless of selectivity
    assert_eq!(stats.filter_strategy, Some(FilterStrategy::Pre));
}

#[test]
fn test_low_selectivity_uses_pre_filter() {
    let mut sing = Singularity::new();

    // 25 concepts, only 3 match (< 0.3 selectivity threshold for datasets > 20)
    for i in 0..3 {
        inject_concept(&mut sing, &format!("match{}", i), "rare", json!("yes"));
    }
    for i in 0..22 {
        inject_concept(&mut sing, &format!("common{}", i), "rare", json!("no"));
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("rare", "yes");

    let _ = sing.find_similar_filtered(&query, 5, &filter);

    let stats = sing.last_retrieval_stats();
    // 3/25 = 0.12 selectivity, should use Pre
    assert_eq!(stats.filter_strategy, Some(FilterStrategy::Pre));
}

#[test]
fn test_medium_selectivity_uses_bucket_post() {
    let mut sing = Singularity::new();

    // 25 concepts, 10 match (0.4 selectivity, in 0.3-0.8 range)
    for i in 0..10 {
        inject_concept(&mut sing, &format!("match{}", i), "level", json!("high"));
    }
    for i in 0..15 {
        inject_concept(&mut sing, &format!("low{}", i), "level", json!("low"));
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("level", "high");

    let _ = sing.find_similar_filtered(&query, 5, &filter);

    let stats = sing.last_retrieval_stats();
    // 10/25 = 0.4 selectivity, should use BucketPost
    assert_eq!(stats.filter_strategy, Some(FilterStrategy::BucketPost));
}

#[test]
fn test_high_selectivity_uses_scan_post() {
    let mut sing = Singularity::new();

    // 25 concepts, 22 match (> 0.8 selectivity threshold)
    for i in 0..22 {
        inject_concept(&mut sing, &format!("major{}", i), "group", json!("a"));
    }
    for i in 0..3 {
        inject_concept(&mut sing, &format!("minor{}", i), "group", json!("b"));
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("group", "a");

    let _ = sing.find_similar_filtered(&query, 5, &filter);

    let stats = sing.last_retrieval_stats();
    // 22/25 = 0.88 selectivity, should use ScanPost
    assert_eq!(stats.filter_strategy, Some(FilterStrategy::ScanPost));
}

#[test]
fn test_complex_filter_with_nested_predicates() {
    let mut sing = Singularity::new();

    // Create concepts with multiple metadata fields
    let c1 = ConceptBuilder::new("c1")
        .with_vector(HVec10240::random())
        .with_metadata("type", json!("doc"))
        .with_metadata("public", json!(true))
        .build()
        .unwrap();
    sing.inject(c1).unwrap();

    let c2 = ConceptBuilder::new("c2")
        .with_vector(HVec10240::random())
        .with_metadata("type", json!("doc"))
        .with_metadata("public", json!(false))
        .build()
        .unwrap();
    sing.inject(c2).unwrap();

    let c3 = ConceptBuilder::new("c3")
        .with_vector(HVec10240::random())
        .with_metadata("type", json!("img"))
        .with_metadata("public", json!(true))
        .build()
        .unwrap();
    sing.inject(c3).unwrap();

    let query = HVec10240::random();

    // Complex filter: (type == "doc" AND public == true)
    let filter = MetadataFilter::and(vec![
        MetadataFilter::eq("type", "doc"),
        MetadataFilter::eq("public", true),
    ]);

    let results = sing.find_similar_filtered(&query, 10, &filter);

    // Only c1 matches
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "c1");
}

#[test]
fn test_find_similar_filtered_respects_top_k() {
    let mut sing = Singularity::new();

    // 10 matching concepts
    for i in 0..10 {
        inject_concept(&mut sing, &format!("doc{}", i), "type", json!("doc"));
    }

    let query = HVec10240::random();
    let filter = MetadataFilter::eq("type", "doc");

    let results = sing.find_similar_filtered(&query, 3, &filter);
    assert!(results.len() <= 3);
}

#[test]
fn test_filtered_results_match_across_strategies() {
    let mut sing = Singularity::new();

    let target_vec = HVec10240::random();
    let query_vec = target_vec.clone();

    let c_target = ConceptBuilder::new("target")
        .with_vector(target_vec.clone())
        .with_metadata("type", json!("target"))
        .build()
        .unwrap();
    sing.inject(c_target).unwrap();

    for i in 0..25 {
        let c = ConceptBuilder::new(&format!("noise{}", i))
            .with_vector(HVec10240::random())
            .with_metadata("type", json!("noise"))
            .build()
            .unwrap();
        sing.inject(c).unwrap();
    }

    let filter = MetadataFilter::eq("type", "target");

    // Selectivity = 1/26 = 0.038 (Pre filter)
    let pre_results = sing.find_similar_filtered(&query_vec, 1, &filter);
    assert_eq!(pre_results.len(), 1);
    assert_eq!(pre_results[0].0, "target");
    assert_eq!(
        sing.last_retrieval_stats().filter_strategy,
        Some(FilterStrategy::Pre)
    );

    // Make it medium selectivity (BucketPost filter)
    for i in 0..10 {
        let c = sing.get(&format!("noise{}", i)).unwrap().clone();
        let c_builder = ConceptBuilder::new(&c.id)
            .with_vector(c.vector.clone())
            .with_metadata("type", json!("target"));
        sing.inject(c_builder.build().unwrap()).unwrap(); // Overwrite
    }

    // Selectivity = 11/26 = 0.42
    let bucket_results = sing.find_similar_filtered(&query_vec, 1, &filter);
    assert_eq!(bucket_results.len(), 1);
    assert_eq!(bucket_results[0].0, "target");
    assert_eq!(
        sing.last_retrieval_stats().filter_strategy,
        Some(FilterStrategy::BucketPost)
    );

    // Make it high selectivity (ScanPost filter)
    for i in 10..24 {
        let c = sing.get(&format!("noise{}", i)).unwrap().clone();
        let c_builder = ConceptBuilder::new(&c.id)
            .with_vector(c.vector.clone())
            .with_metadata("type", json!("target"));
        sing.inject(c_builder.build().unwrap()).unwrap(); // Overwrite
    }

    // Selectivity = 25/26 = 0.96
    let scan_results = sing.find_similar_filtered(&query_vec, 1, &filter);
    assert_eq!(scan_results.len(), 1);
    assert_eq!(scan_results[0].0, "target");
    assert_eq!(
        sing.last_retrieval_stats().filter_strategy,
        Some(FilterStrategy::ScanPost)
    );

    // Ensure the top hit is consistent across all states
    assert_eq!(pre_results[0].0, bucket_results[0].0);
    assert_eq!(bucket_results[0].0, scan_results[0].0);
}
