pub mod concept_builder;
pub mod export_payload;
pub mod framework;
pub mod framework_bridge;
pub mod framework_builder;
pub mod framework_events;
pub mod framework_events_ce;
pub mod framework_metrics;
pub mod framework_namespaces;
pub mod framework_ops;
pub mod framework_persistence;
pub mod framework_ttl;
pub mod framework_validation;
pub mod metadata_filter;
pub mod observability;
pub mod persistence;
pub mod persistence_wasm;
pub mod semantic_bridge;
pub mod semantic_triples;
pub mod singularity;
pub mod singularity_cache;
pub mod singularity_ext;
pub mod singularity_retrieval;
pub mod singularity_search;
pub mod singularity_state;
pub mod singularity_ttl;

pub use framework::ChaoticSemanticFramework;
pub use framework_builder::FrameworkBuilder;
pub use framework_events::MemoryEvent;
pub use semantic_bridge::{
    BridgeConfig, BridgeHit, CanonicalConcept, ConceptGraph, MemoryPacket, ScoreBreakdown,
};
pub use singularity::{Concept, ConceptBuilder};
pub use singularity_retrieval::{CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats};
pub use metadata_filter::MetadataFilter;
pub use persistence::Persistence;

pub mod prelude {
    pub use csm_core::error::{MemoryError, Result};
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::framework_events::MemoryEvent;
    pub use crate::semantic_bridge::{BridgeHit, ConceptGraph, MemoryPacket};
    pub use crate::singularity::{Concept, ConceptBuilder, ConceptDiff, ConceptVersion};
    pub use crate::singularity_retrieval::{
        CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats,
    };
    pub use csm_core::prelude::*;
}
