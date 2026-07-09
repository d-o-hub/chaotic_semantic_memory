#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tests for batch operations API.
#![allow(clippy::cast_possible_truncation)] // Test index-to-char conversion for workflow names

use chaotic_semantic_memory::MemoryError;
use chaotic_semantic_memory::prelude::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

const NS: &str = "_default";

fn make_concept_batch(n: usize) -> Vec<(String, HVec10240)> {
    (0..n)
        .map(|i| (format!("concept_{i}"), HVec10240::random()))
        .collect()
}

fn make_query_batch(n: usize) -> Vec<HVec10240> {
    (0..n).map(|_| HVec10240::random()).collect()
}

#[tokio::test]
async fn inject_concepts_happy_path_in_memory() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concepts = make_concept_batch(5);
    framework.inject_concepts(&concepts).await.unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 5);
}

#[tokio::test]
async fn inject_concepts_empty_batch_returns_ok() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.inject_concepts(&[]).await;
    assert!(result.is_ok());

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 0);
}

#[tokio::test]
async fn inject_concepts_rejects_empty_id() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concepts = vec![
        ("valid_id".to_string(), HVec10240::random()),
        ("".to_string(), HVec10240::random()),
    ];

    let result = framework.inject_concepts(&concepts).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn inject_concepts_rejects_oversized_id() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let long_id = "x".repeat(300);
    let concepts = vec![(long_id, HVec10240::random())];

    let result = framework.inject_concepts(&concepts).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn inject_concepts_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .build()
        .await
        .unwrap();

    let concepts = make_concept_batch(3);
    let _vectors: Vec<HVec10240> = concepts.iter().map(|(_, v)| *v).collect();
    framework.inject_concepts(&concepts).await.unwrap();
    framework.persist().await.unwrap();

    let framework2 = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let stats = framework2.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);

    for (id, vec) in concepts {
        let probe = framework2.probe(vec, 1).await.unwrap();
        assert!(!probe.is_empty());
        assert_eq!(probe[0].0, id);
    }
}

#[tokio::test]
async fn inject_concepts_large_batch() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_concepts(200)
        .build()
        .await
        .unwrap();

    let concepts = make_concept_batch(100);
    framework.inject_concepts(&concepts).await.unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 100);
}

#[tokio::test]
async fn inject_concepts_updates_replaces_existing() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let vec1 = HVec10240::random();
    let vec2 = HVec10240::random();

    framework
        .inject_concepts(&[("same_id".to_string(), vec1)])
        .await
        .unwrap();
    framework
        .inject_concepts(&[("same_id".to_string(), vec2)])
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 1);
}

#[tokio::test]
async fn associate_many_happy_path_in_memory() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concepts = vec![
        ("a".to_string(), HVec10240::random()),
        ("b".to_string(), HVec10240::random()),
        ("c".to_string(), HVec10240::random()),
        ("d".to_string(), HVec10240::random()),
    ];
    framework.inject_concepts(&concepts).await.unwrap();

    let associations = vec![
        ("a".to_string(), "b".to_string(), 0.7),
        ("c".to_string(), "d".to_string(), 0.5),
    ];
    framework.associate_many(&associations).await.unwrap();

    let links_a = framework.get_associations("a").await.unwrap();
    assert_eq!(links_a.len(), 1);
    assert_eq!(links_a[0].0, "b");

    let links_c = framework.get_associations("c").await.unwrap();
    assert_eq!(links_c.len(), 1);
    assert_eq!(links_c[0].0, "d");
}

#[tokio::test]
async fn associate_many_empty_batch_returns_ok() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.associate_many(&[]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn associate_many_rejects_nan_strength() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[
            ("a".to_string(), HVec10240::random()),
            ("b".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let associations = vec![("a".to_string(), "b".to_string(), f32::NAN)];
    let result = framework.associate_many(&associations).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn associate_many_rejects_infinity_strength() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[
            ("a".to_string(), HVec10240::random()),
            ("b".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let associations = vec![("a".to_string(), "b".to_string(), f32::INFINITY)];
    let result = framework.associate_many(&associations).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn associate_many_rejects_negative_strength() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[
            ("a".to_string(), HVec10240::random()),
            ("b".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let associations = vec![("a".to_string(), "b".to_string(), -0.5)];
    let result = framework.associate_many(&associations).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn associate_many_rejects_empty_from_id() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("b".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let associations = vec![("".to_string(), "b".to_string(), 0.5)];
    let result = framework.associate_many(&associations).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn associate_many_rejects_empty_to_id() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("a".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let associations = vec![("a".to_string(), "".to_string(), 0.5)];
    let result = framework.associate_many(&associations).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn associate_many_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[
            ("x".to_string(), HVec10240::random()),
            ("y".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let associations = vec![("x".to_string(), "y".to_string(), 0.8)];
    framework.associate_many(&associations).await.unwrap();
    framework.persist().await.unwrap();

    let framework2 = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let links = framework2.get_associations("x").await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].0, "y");
}

#[tokio::test]
async fn associate_many_large_batch() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_associations_per_concept(100)
        .build()
        .await
        .unwrap();

    let mut concepts = Vec::new();
    for i in 0..50 {
        concepts.push((format!("a_{i}"), HVec10240::random()));
        concepts.push((format!("b_{i}"), HVec10240::random()));
    }
    framework.inject_concepts(&concepts).await.unwrap();

    let mut associations = Vec::new();
    for i in 0..50 {
        associations.push((format!("a_{i}"), format!("b_{i}"), 0.6));
    }
    framework.associate_many(&associations).await.unwrap();

    let links = framework.get_associations("a_0").await.unwrap();
    assert_eq!(links.len(), 1);
}

#[tokio::test]
async fn probe_batch_happy_path_in_memory() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concepts = vec![
        ("apple".to_string(), HVec10240::random()),
        ("banana".to_string(), HVec10240::random()),
        ("cherry".to_string(), HVec10240::random()),
    ];
    framework.inject_concepts(&concepts).await.unwrap();

    let queries = make_query_batch(3);
    let results = framework.probe_batch(&queries, 2).await.unwrap();

    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.len() <= 2);
    }
}

#[tokio::test]
async fn probe_batch_empty_returns_empty_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("test".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let results = framework.probe_batch(&[], 5).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn probe_batch_rejects_zero_top_k() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("test".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let queries = make_query_batch(1);
    let result = framework.probe_batch(&queries, 0).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn probe_batch_rejects_top_k_exceeding_limit() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_probe_top_k(10)
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("test".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let queries = make_query_batch(1);
    let result = framework.probe_batch(&queries, 100).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn probe_batch_finds_exact_match_first() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let target_vec = HVec10240::random();
    framework
        .inject_concepts(&[
            ("target".to_string(), target_vec),
            ("other1".to_string(), HVec10240::random()),
            ("other2".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let queries = vec![target_vec];
    let results = framework.probe_batch(&queries, 1).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].len(), 1);
    assert_eq!(results[0][0].0, "target");
}

#[tokio::test]
async fn probe_batch_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .build()
        .await
        .unwrap();

    let vec1 = HVec10240::random();
    let vec2 = HVec10240::random();
    framework
        .inject_concepts(&[("p1".to_string(), vec1), ("p2".to_string(), vec2)])
        .await
        .unwrap();
    framework.persist().await.unwrap();

    let framework2 = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let queries = vec![vec1, vec2];
    let results = framework2.probe_batch(&queries, 1).await.unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0][0].0, "p1");
    assert_eq!(results[1][0].0, "p2");
}

#[tokio::test]
async fn probe_batch_large_batch() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_concepts(500)
        .build()
        .await
        .unwrap();

    let concepts = make_concept_batch(100);
    framework.inject_concepts(&concepts).await.unwrap();

    let queries = make_query_batch(50);
    let results = framework.probe_batch(&queries, 5).await.unwrap();

    assert_eq!(results.len(), 50);
    for result in &results {
        assert!(result.len() <= 5);
    }
}

#[tokio::test]
async fn probe_batch_cached_happy_path_in_memory() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[
            ("c1".to_string(), HVec10240::random()),
            ("c2".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let queries = make_query_batch(3);
    let results = framework.probe_batch_cached(&queries, 2).await.unwrap();

    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.len() <= 2);
    }
}

#[tokio::test]
async fn probe_batch_cached_empty_returns_empty_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("test".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let results = framework.probe_batch_cached(&[], 5).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn probe_batch_cached_rejects_zero_top_k() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[("test".to_string(), HVec10240::random())])
        .await
        .unwrap();

    let queries = make_query_batch(1);
    let result = framework.probe_batch_cached(&queries, 0).await;
    assert!(matches!(result, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn probe_batch_cached_reuses_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    framework
        .inject_concepts(&[
            ("cached_a".to_string(), HVec10240::random()),
            ("cached_b".to_string(), HVec10240::random()),
        ])
        .await
        .unwrap();

    let query = HVec10240::random();
    let queries = vec![query];

    let first = framework.probe_batch_cached(&queries, 2).await.unwrap();
    let second = framework.probe_batch_cached(&queries, 2).await.unwrap();

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert!(Arc::ptr_eq(&first[0], &second[0]));
}

#[tokio::test]
async fn probe_batch_cached_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .build()
        .await
        .unwrap();

    let vec1 = HVec10240::random();
    framework
        .inject_concepts(&[("cached_p".to_string(), vec1)])
        .await
        .unwrap();
    framework.persist().await.unwrap();

    let framework2 = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .with_concept_cache_size(8)
        .build()
        .await
        .unwrap();

    let queries = vec![vec1];
    let first = framework2.probe_batch_cached(&queries, 1).await.unwrap();
    let second = framework2.probe_batch_cached(&queries, 1).await.unwrap();

    assert!(Arc::ptr_eq(&first[0], &second[0]));
}

#[tokio::test]
async fn probe_batch_cached_multiple_distinct_queries() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    let vec_a = HVec10240::random();
    let vec_b = HVec10240::random();

    framework
        .inject_concepts(&[
            ("multi_a".to_string(), vec_a),
            ("multi_b".to_string(), vec_b),
        ])
        .await
        .unwrap();

    let queries = vec![vec_a, vec_b];
    let results = framework.probe_batch_cached(&queries, 1).await.unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0][0].0, "multi_a");
    assert_eq!(results[1][0].0, "multi_b");

    assert!(!Arc::ptr_eq(&results[0], &results[1]));
}

#[tokio::test]
async fn probe_batch_cached_large_corpus_keeps_exact_semantics() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(64)
        .with_max_batch_size(2001)
        .build()
        .await
        .unwrap();

    let exact = HVec10240::random();
    framework
        .inject_concepts(&[("exact-hit".to_string(), exact)])
        .await
        .unwrap();

    let mut noise = Vec::with_capacity(2_000);
    for i in 0..2_000 {
        noise.push((format!("noise-{i}"), HVec10240::random()));
    }
    framework.inject_concepts(&noise).await.unwrap();

    let queries = vec![exact];
    let first = framework.probe_batch_cached(&queries, 1).await.unwrap();
    let second = framework.probe_batch_cached(&queries, 1).await.unwrap();

    assert_eq!(first[0][0].0, "exact-hit");
    assert_eq!(second[0][0].0, "exact-hit");
    assert!(Arc::ptr_eq(&first[0], &second[0]));
}

#[tokio::test]
async fn batch_operations_concurrent_access() {
    let framework = Arc::new(
        ChaoticSemanticFramework::builder()
            .without_persistence()
            .with_max_concepts(200)
            .build()
            .await
            .unwrap(),
    );

    let mut handles = Vec::new();

    for i in 0..4 {
        let fw = Arc::clone(&framework);
        handles.push(tokio::spawn(async move {
            let concepts = vec![(format!("concurrent_{i}"), HVec10240::random())];
            fw.inject_concepts(&concepts).await.unwrap();

            let queries = make_query_batch(2);
            fw.probe_batch(&queries, 3).await.unwrap();

            Ok::<(), MemoryError>(())
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 4);
}

#[tokio::test]
async fn batch_operations_combined_workflow() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concepts = vec![
        ("workflow_a".to_string(), HVec10240::random()),
        ("workflow_b".to_string(), HVec10240::random()),
        ("workflow_c".to_string(), HVec10240::random()),
    ];
    framework.inject_concepts(&concepts).await.unwrap();

    let associations = vec![
        ("workflow_a".to_string(), "workflow_b".to_string(), 0.7),
        ("workflow_b".to_string(), "workflow_c".to_string(), 0.6),
    ];
    framework.associate_many(&associations).await.unwrap();

    let queries: Vec<HVec10240> = concepts.iter().map(|(_, v)| *v).collect();
    let results = framework.probe_batch(&queries, 2).await.unwrap();

    assert_eq!(results.len(), 3);
    for (i, result) in results.iter().enumerate() {
        assert!(!result.is_empty());
        assert_eq!(
            result[0].0,
            format!("workflow_{}", (b'a' + i as u8) as char)
        );
    }

    let links_a = framework.get_associations("workflow_a").await.unwrap();
    assert_eq!(links_a.len(), 1);
}
