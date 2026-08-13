//! Concept store and singularity engine for chaotic_semantic_memory.
//!
//! This crate provides the core memory engine with:
//! - `Singularity`: Core concept store with similarity search
//! - `Concept`: Core data type with versioning support
//! - `MetadataFilter`: Predicate-based filtering for similarity search
//! - `AnnIndex`: Approximate nearest neighbor index abstraction
//! - `graph_traversal`: BFS, Dijkstra on the concept graph

pub mod concept_builder;
pub mod graph_traversal;
pub mod index;
pub mod index_envelope;
pub mod metadata_filter;
pub mod singularity;
pub mod singularity_cache;
pub mod singularity_decay;
pub mod singularity_ext;
pub mod singularity_retrieval;
pub mod singularity_search;
pub mod singularity_state;
pub mod singularity_ttl;
pub mod singularity_types;

pub use concept_builder::ConceptBuilder;
pub use graph_traversal::TraversalConfig;
pub use index::{AnnIndex, IndexBackend, IndexStats, create_index, validate_index_backend};
pub use metadata_filter::MetadataFilter;
pub use singularity::{
    Association, Concept, ConceptDiff, ConceptVersion, DecayCurve, Singularity, SingularityConfig,
    similarity_cache_key, unix_now_ns, unix_now_secs,
};
pub use singularity_cache::{CacheMetrics, CacheMetricsSnapshot};
pub use singularity_retrieval::{
    CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats, ScoredCandidateParams,
};
pub use singularity_state::NamespaceState;
