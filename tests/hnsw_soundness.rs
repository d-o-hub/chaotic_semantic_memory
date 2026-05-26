#[cfg(feature = "ann-hnsw")]
use chaotic_semantic_memory::index::IndexBackend;
#[cfg(feature = "ann-hnsw")]
use chaotic_semantic_memory::prelude::*;
#[cfg(feature = "ann-hnsw")]
use tempfile::NamedTempFile;

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn test_hnsw_persistence_soundness_miri() {
    let mut vec = HVec10240::zero();
    vec.set_bit(0);
    let query = vec;

    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();

    {
        let framework = FrameworkBuilder::new()
            .with_index_backend(IndexBackend::Hnsw {
                m: 16,
                ef_construction: 100,
                ef_search: 50,
            })
            .build()
            .await
            .unwrap();

        framework.inject_concept("test", vec).await.unwrap();

        // This will call HnswIndex::serialize and write to file
        framework.export_binary(path).await.unwrap();
    }

    // Original framework and its HnswIndex (and the HnswIo loader it might have used) are dropped here.

    let framework2 = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 100,
            ef_search: 50,
        })
        .build()
        .await
        .unwrap();

    // This will call HnswIndex::deserialize from file
    framework2.import_binary(path, false).await.unwrap();

    // Probe should succeed and NOT trigger Miri errors if the transmuted Hnsw
    // is properly backed by the _owner field.
    let results = framework2.probe(query, 1).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "test");
}
