#[cfg(all(feature = "hv-binary", feature = "persistence"))]
mod tests {
    use chaotic_semantic_memory::prelude::*;
    use chaotic_semantic_memory::BHVec10240;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_binary_vector_persistence_roundtrip() {
        let temp = NamedTempFile::new().unwrap();
        let db_path = temp.path().to_str().unwrap();

        // Create framework with binary vectors
        {
            let framework = FrameworkBuilder::new()
                .with_local_db(db_path)
                .with_binary_vectors()
                .build()
                .await
                .unwrap();

            let mut vec = BHVec10240::random();
            framework.inject_concept("bin-test", vec).await.unwrap();
            framework.persist().await.unwrap();
        }

        // Load and verify
        {
            let framework = FrameworkBuilder::new()
                .with_local_db(db_path)
                .with_binary_vectors()
                .build()
                .await
                .unwrap();

            let results = framework.probe(BHVec10240::random(), 10).await.unwrap();
            assert!(results.iter().any(|(id, _)| id == "bin-test"));
        }
    }
}
