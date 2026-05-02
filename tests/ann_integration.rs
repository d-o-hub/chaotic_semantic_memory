use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;

#[tokio::test]
async fn test_hnsw_index_integration() {
    let framework = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        })
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject some concepts
    for i in 0..100 {
        let id = format!("concept-{}", i);
        let mut vec = HVec10240::zero();
        vec.set_bit(i);
        framework.inject_concept(id, vec).await.unwrap();
    }

    // Probe
    let mut query = HVec10240::zero();
    query.set_bit(5); // Should match concept-5
    let results = framework.probe(query, 5).await.unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].0, "concept-5");
    assert!(results[0].1 > 0.99);
}

#[tokio::test]
async fn test_hnsw_update_replaces_previous_vector() {
    let framework = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        })
        .without_persistence()
        .build()
        .await
        .unwrap();

    let mut original = HVec10240::zero();
    for bit in 0..4000 {
        original.set_bit(bit);
    }

    let mut near_original = HVec10240::zero();
    for bit in 0..3999 {
        near_original.set_bit(bit);
    }

    let mut updated = HVec10240::zero();
    for bit in 6000..10000 {
        updated.set_bit(bit);
    }

    framework.inject_concept("target", original).await.unwrap();
    framework
        .inject_concept("decoy", near_original)
        .await
        .unwrap();
    framework.inject_concept("target", updated).await.unwrap();

    let results = framework.probe(original, 2).await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].0, "decoy");
}

#[tokio::test]
async fn test_lsh_index_integration() {
    let framework = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 5,
            hash_bits: 16,
        })
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject some concepts
    for i in 0..100 {
        let id = format!("concept-{}", i);
        let mut vec = HVec10240::zero();
        vec.set_bit(i);
        framework.inject_concept(id, vec).await.unwrap();
    }

    // Probe
    let mut query = HVec10240::zero();
    query.set_bit(10); // Should match concept-10
    let results = framework.probe(query, 5).await.unwrap();

    assert!(!results.is_empty());
    // LSH might not be as perfect as HNSW on this tiny set if bits aren't sampled,
    // but it should likely find it among the top results.
    assert!(results.iter().any(|(id, _)| id == "concept-10"));
}
