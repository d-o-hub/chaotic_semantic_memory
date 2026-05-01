//! ANN Index traits and backends (ADR-0068).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::error::Result;
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;

pub mod brute_force;
#[cfg(feature = "ann-hnsw")]
pub mod hnsw;
#[cfg(feature = "ann-lsh")]
pub mod lsh;

/// Statistics for an ANN index.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Hnsw {
        /// Number of bi-directional links for each element (default: 16).
        m: usize,
        /// Size of the dynamic list for the nearest neighbors (default: 200).
        ef_construction: usize,
        /// Size of the dynamic list for the nearest neighbors during search (default: 50).
        ef_search: usize,
    },
    /// Locality-Sensitive Hashing (LSH) index.
    Lsh {
        /// Number of hash tables.
        num_tables: usize,
        /// Number of hash bits per table.
        hash_bits: usize,
    },
}

/// Trait for Approximate Nearest Neighbor (ANN) indices.
pub trait AnnIndex: Send + Sync + Debug {
    /// Insert a concept into the index.
    fn insert(&mut self, id: String, vec: &HVec10240) -> Result<()>;

    /// Delete a concept from the index.
    fn delete(&mut self, id: &str) -> Result<()>;

    /// Search for the top-k nearest neighbors.
    fn search(&self, query: &HVec10240, top_k: usize) -> Result<Vec<(String, f32)>>;

    /// Rebuild the index from scratch using all concepts.
    fn rebuild(&mut self, concepts: &HashMap<String, Concept>) -> Result<()>;

    /// Get statistics for the index.
    fn stats(&self) -> IndexStats;

    /// Serialize the index state for persistence.
    fn serialize(&self) -> Result<Vec<u8>>;

    /// Deserialize the index state from persistence.
    fn deserialize(&mut self, data: &[u8]) -> Result<()>;
}
