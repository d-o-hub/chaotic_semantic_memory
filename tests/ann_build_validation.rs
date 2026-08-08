//! ADR-0093: invalid ANN backend configuration must fail the framework build
//! with `MemoryError::InvalidInput` instead of panicking or silently degrading.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(any(feature = "ann-hnsw", feature = "ann-lsh"))]

use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn build_rejects_zero_m_hnsw_backend() {
    let err = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 0,
            ef_construction: 200,
            ef_search: 50,
        })
        .build()
        .await
        .err()
        .expect("m=0 must fail build");
    assert!(
        matches!(err, MemoryError::InvalidInput { .. }),
        "expected InvalidInput, got {err}"
    );
}

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn build_rejects_m_above_hnsw_rs_limit() {
    let err = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 257,
            ef_construction: 200,
            ef_search: 50,
        })
        .build()
        .await
        .err()
        .expect("m=257 must fail build (hnsw_rs hard-exits above 256)");
    assert!(
        matches!(err, MemoryError::InvalidInput { .. }),
        "expected InvalidInput, got {err}"
    );
}

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn build_rejects_zero_ef_construction() {
    let err = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 0,
            ef_search: 50,
        })
        .build()
        .await
        .err()
        .expect("ef_construction=0 must fail build");
    assert!(
        matches!(err, MemoryError::InvalidInput { .. }),
        "expected InvalidInput, got {err}"
    );
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn build_rejects_zero_num_tables_lsh_backend() {
    let err = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 0,
            hash_bits: 16,
        })
        .build()
        .await
        .err()
        .expect("num_tables=0 must fail build");
    assert!(
        matches!(err, MemoryError::InvalidInput { .. }),
        "expected InvalidInput, got {err}"
    );
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn build_rejects_hash_bits_above_64_lsh_backend() {
    let err = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 5,
            hash_bits: 65,
        })
        .build()
        .await
        .err()
        .expect("hash_bits=65 must fail build instead of being clamped");
    assert!(
        matches!(err, MemoryError::InvalidInput { .. }),
        "expected InvalidInput, got {err}"
    );
}

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn valid_hnsw_backend_builds_and_indexes() {
    let framework = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        })
        .build()
        .await
        .expect("valid HNSW config must build");

    let id = "hnsw-1".to_string();
    let mut vec = HVec10240::zero();
    vec.set_bit(3);
    framework.inject_concept(id.clone(), vec).await.unwrap();

    let mut query = HVec10240::zero();
    query.set_bit(3);
    let results = framework.probe(query, 1).await.unwrap();
    assert_eq!(results[0].0, id);
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn valid_lsh_backend_builds_and_indexes() {
    let framework = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 5,
            hash_bits: 16,
        })
        .build()
        .await
        .expect("valid LSH config must build");

    let id = "lsh-1".to_string();
    let mut vec = HVec10240::zero();
    vec.set_bit(7);
    framework.inject_concept(id.clone(), vec).await.unwrap();

    let mut query = HVec10240::zero();
    query.set_bit(7);
    let results = framework.probe(query, 1).await.unwrap();
    assert_eq!(results[0].0, id);
}
