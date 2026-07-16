//! ANN backend validation at framework build time (ADR-0093 / Wave 32 P0).
//!
//! Invalid public ANN configuration must return `Err(InvalidInput)`, never panic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(any(feature = "ann-hnsw", feature = "ann-lsh"))]

use chaotic_semantic_memory::index::IndexBackend;
use chaotic_semantic_memory::prelude::*;

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn build_rejects_invalid_hnsw_m_zero() {
    let result = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 0,
            ef_construction: 200,
            ef_search: 50,
        })
        .without_persistence()
        .build()
        .await;

    match result {
        Err(MemoryError::InvalidInput { field, .. }) => assert_eq!(field, "m"),
        Err(e) => panic!("expected InvalidInput for HNSW m=0, got {e}"),
        Ok(_) => panic!("expected InvalidInput for HNSW m=0, build succeeded"),
    }
}

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn build_rejects_invalid_hnsw_m_too_large() {
    let result = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 257,
            ef_construction: 200,
            ef_search: 50,
        })
        .without_persistence()
        .build()
        .await;

    match result {
        Err(MemoryError::InvalidInput { field, .. }) => assert_eq!(field, "m"),
        Err(e) => panic!("expected InvalidInput for HNSW m=257, got {e}"),
        Ok(_) => panic!("expected InvalidInput for HNSW m=257, build succeeded"),
    }
}

#[cfg(feature = "ann-hnsw")]
#[tokio::test]
async fn build_valid_hnsw_ok() {
    let fw = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Hnsw {
            m: 16,
            ef_construction: 100,
            ef_search: 50,
        })
        .without_persistence()
        .build()
        .await
        .expect("valid HNSW must build");
    fw.inject_concept("c1", HVec10240::random())
        .await
        .expect("inject on HNSW");
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn build_rejects_invalid_lsh_num_tables() {
    let result = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 0,
            hash_bits: 8,
        })
        .without_persistence()
        .build()
        .await;

    match result {
        Err(MemoryError::InvalidInput { field, .. }) => assert_eq!(field, "num_tables"),
        Err(e) => panic!("expected InvalidInput for LSH num_tables=0, got {e}"),
        Ok(_) => panic!("expected InvalidInput for LSH num_tables=0, build succeeded"),
    }
}

#[cfg(feature = "ann-lsh")]
#[tokio::test]
async fn build_valid_lsh_ok() {
    let fw = FrameworkBuilder::new()
        .with_index_backend(IndexBackend::Lsh {
            num_tables: 4,
            hash_bits: 8,
        })
        .without_persistence()
        .build()
        .await
        .expect("valid LSH must build");
    fw.inject_concept("c1", HVec10240::random())
        .await
        .expect("inject on LSH");
}
