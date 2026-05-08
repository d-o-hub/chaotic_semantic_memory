#![cfg(any(feature = "ann-hnsw", feature = "ann-lsh"))]

use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;

const NS: &str = "_default";

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn test_hnsw_index_integration() {
    let framework: ChaoticSemanticFramework<HVec10240> = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        })
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject some concepts (using random to create distinct vectors)
    let mut injected = Vec::new();
    for i in 0..100 {
        let id = format!("concept-{}", i);
        let vec = HVec10240::random();
        injected.push(vec);
        framework.inject_concept(id, vec).await.unwrap();
    }

    // Probe with a copy of one of the injected vectors
    let query = injected[5];
    let results = framework.probe(query, 5).await.unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].0, "concept-5");
    assert!(results[0].1 > 0.99);
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn test_lsh_index_integration() {
    let framework: ChaoticSemanticFramework<HVec10240> = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 5,
            hash_bits: 16,
        })
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject some concepts (using random to create distinct vectors)
    let mut injected = Vec::new();
    for i in 0..100 {
        let id = format!("concept-{}", i);
        let vec = HVec10240::random();
        injected.push(vec);
        framework.inject_concept(id, vec).await.unwrap();
    }

    // Probe with a copy of one of the injected vectors
    let query = injected[10];
    let results = framework.probe(query, 5).await.unwrap();

    assert!(!results.is_empty());
    // LSH might not be as perfect as HNSW on this tiny set if bits aren't sampled,
    // but it should likely find it among the top results.
    assert!(results.iter().any(|(id, _)| id == "concept-10"));
}
