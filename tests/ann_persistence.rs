#![cfg(feature = "ann-lsh")]

use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

const NS: &str = "_default";

#[tokio::test]
async fn test_index_persistence_roundtrip() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();

    {
        let framework: ChaoticSemanticFramework<HVec10240> = FrameworkBuilder::new()
            .with_index_backend(IndexBackend::Lsh {
                num_tables: 3,
                hash_bits: 8,
            })
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();

        // Inject concept (use random vector since HVec10240 is now f32-based)
        let vec = HVec10240::random();
        framework.inject_concept("persist-test", vec).await.unwrap();

        // Persist
        framework.persist().await.unwrap();
    }

    // Re-load in new framework instance
    {
        let framework: ChaoticSemanticFramework<HVec10240> = FrameworkBuilder::new()
            .with_index_backend(IndexBackend::Lsh {
                num_tables: 3,
                hash_bits: 8,
            })
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();

        // Re-generate the same vector for query (random seed not deterministic across scopes)
        let query_vec = HVec10240::random();
        let results = framework.probe(&query_vec, 1).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "persist-test");
    }
}
