//! Concept store and singularity engine for chaotic_semantic_memory.
//!
//! This crate provides the core memory engine with:
//! - `Singularity`: Core concept store with similarity search
//! - `Concept`: Core data type with versioning support
//! - `MetadataFilter`: Predicate-based filtering for similarity search
//! - `AnnIndex`: Approximate nearest neighbor index abstraction
//! - `graph_traversal`: BFS, Dijkstra on the concept graph

mod concept_builder;
mod graph_traversal;
mod index;
mod metadata_filter;
mod singularity;
mod singularity_cache;
mod singularity_ext;
mod singularity_retrieval;
mod singularity_search;
mod singularity_state;
mod singularity_ttl;

pub use concept_builder::ConceptBuilder;
pub use graph_traversal::TraversalConfig;
pub use index::{AnnIndex, IndexBackend, IndexStats};
pub use metadata_filter::MetadataFilter;
pub use singularity::{
    Concept, ConceptDiff, ConceptVersion, Singularity, SingularityConfig, unix_now_ns,
    unix_now_secs,
};
pub use singularity_cache::{CacheMetrics, CacheMetricsSnapshot};
pub use singularity_retrieval::{CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats};
pub use singularity_state::NamespaceState;
