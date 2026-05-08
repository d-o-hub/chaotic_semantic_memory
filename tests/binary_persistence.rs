#[cfg(all(feature = "hv-binary", feature = "persistence"))]
mod tests {
    use chaotic_semantic_memory::prelude::*;
    use chaotic_semantic_memory::BHVec10240;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_binary_vector_persistence_roundtrip() {
        let temp = NamedTempFile::new().unwrap();
        let db_path = temp.path().to_str().unwrap();
        let original_vec = BHVec10240::random();

        // Create framework with binary vectors
        {
            let framework = FrameworkBuilder::new()
                .with_local_db(db_path)
                .with_binary_vectors()
                .build()
                .await
                .unwrap();

            framework.inject_concept("bin-test", original_vec).await.unwrap();
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

            let concept = framework.get_concept("bin-test").await.unwrap().unwrap();
            assert_eq!(concept.vector, original_vec);

            let results = framework.probe(original_vec, 1).await.unwrap();
            assert_eq!(results[0].0, "bin-test");
        }
    }
}
