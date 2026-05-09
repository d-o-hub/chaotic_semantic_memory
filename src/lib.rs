//! Chaotic Semantic Memory (CSM)
//!
//! A high-performance, hyperdimensional computing (HDC) based memory system
//! for AI agents and applications.

pub mod error;
pub mod hyperdim;
pub mod singularity;
pub mod concept_builder;
pub mod encoder;
pub mod framework;
pub mod framework_builder;
pub mod framework_events;
pub mod framework_metrics;
pub mod metadata_filter;
pub mod persistence;
pub mod reservoir;
pub mod retrieval;
pub mod semantic_bridge;
pub mod semantic_triples;
pub mod graph_traversal;
pub mod cli;

// Internal modules used by others
pub(crate) mod bundle;
pub(crate) mod singularity_cache;
pub(crate) mod singularity_retrieval;
pub(crate) mod singularity_state;
pub(crate) mod singularity_ttl;
pub(crate) mod singularity_ext;
pub(crate) mod singularity_search;
pub(crate) mod reservoir_sparse;
pub(crate) mod reservoir_inertial;
pub(crate) mod index;
pub mod embedding;
pub(crate) mod bridge_retrieval;
pub(crate) mod export_payload;

// Persistence submodules
pub(crate) mod persistence_concepts;
pub(crate) mod persistence_index;
pub(crate) mod persistence_migrations;
pub(crate) mod persistence_ops;
pub(crate) mod persistence_versions;
pub(crate) mod persistence_wasm;

// Framework submodules
pub(crate) mod framework_bridge;
pub(crate) mod framework_graph_rag;
pub(crate) mod framework_namespaces;
pub(crate) mod framework_ops;
pub(crate) mod framework_persistence;
pub(crate) mod framework_rerank;
pub(crate) mod framework_ttl;
pub(crate) mod framework_validation;

pub use error::{MemoryError, Result};
pub use hyperdim::{HVec10240, Hypervector};
#[cfg(feature = "hv-binary")]
pub use hyperdim::BHVec10240;
pub use singularity::{Concept, Singularity, SingularityConfig};
pub use concept_builder::ConceptBuilder;
pub use encoder::{TextEncoder, TextEncoderConfig};
pub use framework::ChaoticSemanticFramework;
pub use framework_builder::FrameworkBuilder;
pub use metadata_filter::MetadataFilter;

/// Prelude for common Chaotic Semantic Memory types
pub mod prelude {
    pub use crate::error::{MemoryError, Result};
    pub use crate::hyperdim::{HVec10240, Hypervector};
    #[cfg(feature = "hv-binary")]
    pub use crate::hyperdim::BHVec10240;
    pub use crate::singularity::{Concept, Singularity, SingularityConfig};
    pub use crate::concept_builder::ConceptBuilder;
    pub use crate::encoder::{TextEncoder, TextEncoderConfig};
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::metadata_filter::MetadataFilter;
}
