#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn
)]
//!
//! High-performance memory system using **Hyperdimensional Computing** (HDC) and
//! chaotic echo-state reservoir dynamics.

pub use bridge_retrieval::BridgeRetrieval;
pub use bundle::BundleAccumulator;
pub use error::{MemoryError, Result};
pub use framework::ChaoticSemanticFramework;
pub use framework_builder::FrameworkBuilder;
pub use framework_events::MemoryEvent;
pub use hyperdim::{HVec10240, Hypervector, batch_cosine_similarity};
#[cfg(feature = "hv-binary")]
pub use hyperdim::BHVec10240;
pub use semantic_bridge::{
    BridgeConfig, BridgeHit, CanonicalConcept, ConceptGraph, MemoryPacket, ScoreBreakdown,
};
pub use singularity::{Concept, ConceptBuilder};
pub use singularity_retrieval::{CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats};

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod bridge_persistence;
pub mod bridge_retrieval;
pub mod bundle;
#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
pub mod cli;
pub mod concept_builder;
pub mod embedding;
pub mod encoder;
pub mod error;
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
pub mod hyperdim;
#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
pub mod mcp;
pub mod metadata_filter;
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
pub mod reservoir;
mod reservoir_inertial;
mod reservoir_sparse;
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

// Stub persistence module when persistence feature is disabled on non-WASM
#[cfg(all(not(target_arch = "wasm32"), not(feature = "persistence")))]
pub mod persistence {
    use crate::error::Result;
    use crate::hyperdim::{HVec10240, Hypervector};
    use crate::singularity::Concept;

    #[derive(Debug)]
    pub struct Persistence;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(bound = "H: Hypervector")]
    pub struct ConceptVersion<H: Hypervector = HVec10240> {
        pub concept_id: String,
        pub version: i64,
        pub vector: H,
        pub metadata: serde_json::Value,
        pub modified_at: u64,
    }

    impl Persistence {
        pub async fn save_concept<H: Hypervector + 'static>(&self, _ns: &str, _concept: &Concept<H>) -> Result<()> { Ok(()) }
        pub async fn save_concepts<H: Hypervector + 'static>(&self, _ns: &str, _concepts: &[Concept<H>]) -> Result<()> { Ok(()) }
        pub async fn load_concept<H: Hypervector + 'static>(&self, _ns: &str, _id: &str) -> Result<Option<Concept<H>>> { Ok(None) }
        pub async fn load_all_concepts<H: Hypervector + 'static>(&self, _ns: &str) -> Result<Vec<Concept<H>>> { Ok(Vec::new()) }
        pub async fn delete_concept(&self, _ns: &str, _id: &str) -> Result<()> { Ok(()) }
        pub async fn save_association(&self, _ns: &str, _from: &str, _to: &str, _strength: f32) -> Result<()> { Ok(()) }
        pub async fn load_associations(&self, _ns: &str, _id: &str) -> Result<Vec<(String, f32)>> { Ok(Vec::new()) }
        pub async fn clear_all(&self) -> Result<()> { Ok(()) }
        pub async fn checkpoint(&self) -> Result<()> { Ok(()) }
        pub async fn health_check(&self) -> Result<()> { Ok(()) }
        pub async fn size(&self) -> Result<u64> { Ok(0) }
        pub async fn get_concept_history<H: Hypervector + 'static>(&self, _ns: &str, _id: &str, _limit: usize) -> Result<Vec<ConceptVersion<H>>> { Ok(Vec::new()) }
        pub async fn schema_version(&self) -> Result<i64> { Ok(0) }
        pub async fn save_index(&self, _ns: &str, _id: &str, _data: &[u8]) -> Result<()> { Ok(()) }
        pub async fn load_index(&self, _ns: &str, _id: &str) -> Result<Option<Vec<u8>>> { Ok(None) }
        pub async fn list_namespaces(&self) -> Result<Vec<String>> { Ok(vec!["_default".to_string()]) }
        pub async fn clear_namespace(&self, _ns: &str) -> Result<()> { Ok(()) }
    }
}

pub mod prelude {
    pub use crate::bridge_retrieval::BridgeRetrieval;
    pub use crate::bundle::BundleAccumulator;
    pub use crate::error::{MemoryError, Result};
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::framework_events::MemoryEvent;
    pub use crate::hyperdim::{HVec10240, Hypervector};
    #[cfg(feature = "hv-binary")]
    pub use crate::hyperdim::BHVec10240;
    pub use crate::semantic_bridge::{BridgeHit, ConceptGraph, MemoryPacket};
    pub use crate::singularity::{Concept, ConceptBuilder};
    pub use crate::singularity_retrieval::{
        CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats,
    };
}

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(any(target_arch = "wasm32", test))]
mod wasm_ext;
#[cfg(any(target_arch = "wasm32", test))]
mod wasm_graph_rag;
