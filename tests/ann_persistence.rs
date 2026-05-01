use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_index_persistence_roundtrip() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap();

    {
        let framework = FrameworkBuilder::new()
            .with_index_backend(IndexBackend::Lsh {
                num_tables: 3,
                hash_bits: 8,
            })
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();

        // Inject concept
        let mut vec = HVec10240::zero();
        vec.set_bit(42);
        framework.inject_concept("persist-test", vec).await.unwrap();

        // Persist
        framework.persist().await.unwrap();
    }

    // Re-load in new framework instance
    {
        let framework = FrameworkBuilder::new()
            .with_index_backend(IndexBackend::Lsh {
                num_tables: 3,
                hash_bits: 8,
            })
            .with_local_db(db_path)
            .build()
            .await
            .unwrap();

        // Probe - should use loaded/rebuilt index
        let mut query = HVec10240::zero();
        query.set_bit(42);
        let results = framework.probe(query, 1).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "persist-test");
    }
}
