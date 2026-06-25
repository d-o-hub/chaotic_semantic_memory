//! ANN Index traits and backends (ADR-0068).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::singularity::Concept;
use csm_core::error::Result;
use csm_core::hyperdim::{HVec10240, Hypervector};

pub mod brute_force;
#[cfg(feature = "ann-hnsw")]
pub mod hnsw;
#[cfg(feature = "ann-lsh")]
pub mod lsh;
pub mod lsh_query_mod;

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
