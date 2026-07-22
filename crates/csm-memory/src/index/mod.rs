//! ANN Index traits and backends (ADR-0068).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::singularity::Concept;
use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::{HVec10240, Hypervector};

pub mod brute_force;
#[cfg(feature = "ann-hnsw")]
pub mod hnsw;
#[cfg(feature = "ann-lsh")]
pub mod lsh;

/// Statistics for an ANN index.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexStats {
    pub backend: String,
    pub count: usize,
    pub memory_usage_bytes: usize,
}

/// Supported ANN index backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum IndexBackend {
    /// Exact search via linear scan.
    #[default]
    BruteForce,
    /// Hierarchical Navigable Small Worlds (HNSW) index.
    #[cfg(feature = "ann-hnsw")]
    Hnsw {
        /// Number of bi-directional links for each element (default: 16).
        m: usize,
        /// Size of the dynamic list for the nearest neighbors (default: 200).
        ef_construction: usize,
        /// Size of the dynamic list for the nearest neighbors during search (default: 50).
        ef_search: usize,
    },
    /// Locality-Sensitive Hashing (LSH) index.
    #[cfg(feature = "ann-lsh")]
    Lsh {
        /// Number of hash tables.
        num_tables: usize,
        /// Number of hash bits per table.
        hash_bits: usize,
    },
}

/// Trait for Approximate Nearest Neighbor (ANN) indices.
pub trait AnnIndex<H: Hypervector = HVec10240>: Send + Sync + Debug + 'static {
    /// Insert a concept into the index.
    fn insert(&mut self, id: String, vec: &H) -> Result<()>;

    /// Delete a concept from the index.
    fn delete(&mut self, id: &str) -> Result<()>;

    /// Search for the top-k nearest neighbors.
    fn search(&self, query: &H, top_k: usize) -> Result<Vec<(String, f32)>>;

    /// Search for the nearest neighbors with a metadata filter.
    fn search_filtered(
        &self,
        query: &H,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
        concepts: &std::collections::HashMap<String, crate::singularity::Concept<H>>,
    ) -> Result<Vec<(String, f32)>>;

    /// Rebuild the index from scratch using all concepts.
    fn rebuild(&mut self, concepts: &HashMap<String, Concept<H>>) -> Result<()>;

    /// Get statistics for the index.
    fn stats(&self) -> IndexStats;

    /// Serialize the index state for persistence.
    fn serialize(&self) -> Result<Vec<u8>>;

    /// Deserialize the index state from persistence.
    fn deserialize(&mut self, data: &[u8]) -> Result<()>;
}

/// Create an ANN index backend based on configuration.
pub fn create_index<H: Hypervector + 'static>(
    backend: &IndexBackend,
) -> Result<Box<dyn AnnIndex<H>>> {
    let index: Box<dyn AnnIndex<H>> = match backend {
        IndexBackend::BruteForce => Box::new(brute_force::BruteForce::new()),
        #[cfg(feature = "ann-hnsw")]
        IndexBackend::Hnsw {
            m,
            ef_construction,
            ef_search,
        } => Box::new(hnsw::HnswIndex::new(*m, *ef_construction, *ef_search)?),
        #[cfg(feature = "ann-lsh")]
        IndexBackend::Lsh {
            num_tables,
            hash_bits,
        } => Box::new(lsh::LshIndex::new(*num_tables, *hash_bits)?),
        #[allow(unreachable_patterns)]
        _ => Box::new(brute_force::BruteForce::new()),
    };
    Ok(index)
}

/// Validate that `backend` can be constructed.
///
/// Builds a throwaway [`HVec10240`] index via [`create_index`] so checks stay in
/// lock-step with constructors (HNSW `m` ∈ [1, 256], LSH `num_tables > 0`, etc.).
/// Call at framework build time so invalid configs fail closed with
/// `MemoryError::InvalidInput` rather than panicking later.
pub fn validate_index_backend(backend: &IndexBackend) -> Result<()> {
    let _index: Box<dyn AnnIndex<HVec10240>> = create_index(backend)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    #[cfg(any(feature = "ann-hnsw", feature = "ann-lsh"))]
    use csm_core_lib::error::MemoryError;

    #[test]
    fn create_index_bruteforce_ok() {
        let idx = create_index::<HVec10240>(&IndexBackend::BruteForce);
        assert!(idx.is_ok());
        assert!(validate_index_backend(&IndexBackend::BruteForce).is_ok());
    }

    #[cfg(feature = "ann-hnsw")]
    #[test]
    fn create_index_hnsw_valid_ok() {
        let backend = IndexBackend::Hnsw {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        };
        assert!(create_index::<HVec10240>(&backend).is_ok());
        assert!(validate_index_backend(&backend).is_ok());
    }

    #[cfg(feature = "ann-hnsw")]
    #[test]
    fn create_index_hnsw_m_zero_is_invalid_input() {
        let backend = IndexBackend::Hnsw {
            m: 0,
            ef_construction: 200,
            ef_search: 50,
        };
        match create_index::<HVec10240>(&backend) {
            Err(MemoryError::InvalidInput { field, .. }) => assert_eq!(field, "m"),
            other => panic!("expected InvalidInput for m=0, got {other:?}"),
        }
        assert!(matches!(
            validate_index_backend(&backend),
            Err(MemoryError::InvalidInput { .. })
        ));
    }

    #[cfg(feature = "ann-hnsw")]
    #[test]
    fn create_index_hnsw_m_too_large_is_invalid_input() {
        let backend = IndexBackend::Hnsw {
            m: 257,
            ef_construction: 200,
            ef_search: 50,
        };
        match create_index::<HVec10240>(&backend) {
            Err(MemoryError::InvalidInput { field, .. }) => assert_eq!(field, "m"),
            other => panic!("expected InvalidInput for m=257, got {other:?}"),
        }
    }

    #[cfg(feature = "ann-lsh")]
    #[test]
    fn create_index_lsh_valid_ok() {
        let backend = IndexBackend::Lsh {
            num_tables: 4,
            hash_bits: 8,
        };
        assert!(create_index::<HVec10240>(&backend).is_ok());
        assert!(validate_index_backend(&backend).is_ok());
    }

    #[cfg(feature = "ann-lsh")]
    #[test]
    fn create_index_lsh_zero_tables_is_invalid_input() {
        let backend = IndexBackend::Lsh {
            num_tables: 0,
            hash_bits: 8,
        };
        match create_index::<HVec10240>(&backend) {
            Err(MemoryError::InvalidInput { field, .. }) => assert_eq!(field, "num_tables"),
            other => panic!("expected InvalidInput for num_tables=0, got {other:?}"),
        }
        assert!(matches!(
            validate_index_backend(&backend),
            Err(MemoryError::InvalidInput { .. })
        ));
    }
}
