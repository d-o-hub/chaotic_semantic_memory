#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn
)]

pub use bridge_retrieval::BridgeRetrieval;
pub use csm_core::bundle::BundleAccumulator;
pub use csm_core::error::{MemoryError, Result};
pub use csm_core::hyperdim::{HVec10240, batch_cosine_similarity};
pub use framework::ChaoticSemanticFramework;
pub use framework_builder::FrameworkBuilder;
pub use framework_events::MemoryEvent;
pub use semantic_bridge::{
    BridgeConfig, BridgeHit, CanonicalConcept, ConceptGraph, MemoryPacket, ScoreBreakdown,
};
pub use singularity::{Concept, ConceptBuilder};
pub use singularity_retrieval::{CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats};

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod bridge_persistence;
pub mod bridge_retrieval;
pub use csm_core::bundle;
#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
pub mod cli;
pub mod concept_builder;
pub mod embedding;
pub use csm_core::encoder;
pub use csm_core::error;
mod export_payload;
pub mod framework;
mod framework_bridge;
pub mod framework_builder;
mod framework_events;
pub mod framework_events_ce;
mod framework_graph_rag;
mod framework_metrics;
mod framework_namespaces;
#[cfg(not(target_arch = "wasm32"))]
mod framework_ops;
mod framework_persistence;
mod framework_ttl;
mod framework_validation;
pub mod graph_traversal;
pub use csm_core::hyperdim;
#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
pub mod mcp;
pub mod metadata_filter;
#[cfg(any(feature = "prometheus", feature = "otlp-json"))]
pub mod observability;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_concepts;
pub mod semantic_triples;
pub use metadata_filter::MetadataFilter;
pub mod index;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub mod persistence;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_index;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_migrations;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_ops;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_versions;
#[cfg(target_arch = "wasm32")]
pub mod persistence_wasm;
pub use csm_core::reservoir;
pub mod retrieval;
pub mod semantic_bridge;
pub mod singularity;
mod singularity_cache;
mod singularity_ext;
mod singularity_retrieval;
mod singularity_search;
pub mod singularity_state;
mod singularity_ttl;

#[cfg(target_arch = "wasm32")]
pub use crate::persistence_wasm as persistence;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "persistence")))]
pub mod persistence {
    use csm_core::Result;
    use crate::singularity::Concept;
    #[derive(Debug)]
    pub struct Persistence;
    pub use crate::singularity::ConceptVersion;
    impl Persistence {
        pub async fn save_concept(&self, _ns: &str, _concept: &Concept) -> Result<()> { Ok(()) }
        pub async fn save_concepts(&self, _ns: &str, _concepts: &[Concept]) -> Result<()> { Ok(()) }
        pub async fn load_concept(&self, _ns: &str, _id: &str) -> Result<Option<Concept>> { Ok(None) }
        pub async fn load_all_concepts(&self, _ns: &str) -> Result<Vec<Concept>> { Ok(Vec::new()) }
        pub async fn delete_concept(&self, _ns: &str, _id: &str) -> Result<()> { Ok(()) }
        pub async fn save_association(&self, _ns: &str, _from: &str, _to: &str, _strength: f32) -> Result<()> { Ok(()) }
        pub async fn save_associations(&self, _ns: &str, _associations: &[(String, String, f32)]) -> Result<()> { Ok(()) }
        pub async fn load_associations(&self, _ns: &str, _id: &str) -> Result<Vec<(String, f32)>> { Ok(Vec::new()) }
        pub async fn delete_association(&self, _ns: &str, _from: &str, _to: &str) -> Result<()> { Ok(()) }
        pub async fn clear_concept_associations(&self, _ns: &str, _id: &str) -> Result<()> { Ok(()) }
        pub async fn clear_all(&self) -> Result<()> { Ok(()) }
        pub async fn checkpoint(&self) -> Result<()> { Ok(()) }
        pub async fn health_check(&self) -> Result<()> { Ok(()) }
        pub async fn size(&self) -> Result<u64> { Ok(0) }
        pub async fn backup(&self, _path: &str) -> Result<()> { Ok(()) }
        pub async fn restore(&self, _path: &str) -> Result<()> { Ok(()) }
        pub async fn get_version_scoped(&self, _ns: &str, _id: &str, _version: u64) -> Result<Option<Concept>> { Ok(None) }
        pub async fn list_versions_scoped(&self, _ns: &str, _id: &str) -> Result<Vec<crate::singularity::ConceptVersion>> { Ok(Vec::new()) }
        pub async fn get_concept_history(&self, _ns: &str, _id: &str, _limit: usize) -> Result<Vec<ConceptVersion>> { Ok(Vec::new()) }
        pub async fn schema_version(&self) -> Result<i64> { Ok(0) }
        pub async fn save_index(&self, _ns: &str, _id: &str, _data: &[u8]) -> Result<()> { Ok(()) }
        pub async fn load_index(&self, _ns: &str, _id: &str) -> Result<Option<Vec<u8>>> { Ok(None) }
        pub async fn apply_migrations(&self, _target_version: i64) -> Result<()> { Ok(()) }
        pub async fn list_namespaces(&self) -> Result<Vec<String>> { Ok(Vec::new()) }
        pub async fn clear_namespace(&self, _ns: &str) -> Result<()> { Ok(()) }
    }
}

pub mod prelude {
    pub use csm_core::error::{MemoryError, Result};
    pub use crate::bridge_retrieval::BridgeRetrieval;
    pub use csm_core::bundle::BundleAccumulator;
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::framework_events::MemoryEvent;
    pub use csm_core::hyperdim::HVec10240;
    pub use crate::semantic_bridge::{BridgeHit, ConceptGraph, MemoryPacket};
    pub use crate::singularity::{Concept, ConceptBuilder, ConceptDiff, ConceptVersion};
    pub use crate::singularity_retrieval::{
        CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats,
    };
}

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_ext;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wasm_ext;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wasm_ext_tests;
#[cfg(target_arch = "wasm32")]
mod wasm_graph_rag;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wasm_graph_rag;
