//! # Chaotic Semantic Memory
//!
//! High-performance memory system using **Hyperdimensional Computing** (HDC) and
//! chaotic echo-state reservoir dynamics.
//!
//! ## Encoding Model
//!
//! This crate uses HDC (not transformer embeddings) for text-to-vector encoding.
//! [`encoder::TextEncoder`] converts text via token-level FNV-1a hashing, positional
//! permutation, and majority-rule bundling into 10240-bit binary hypervectors.
//! Similar tokens in similar positions produce similar vectors. For true semantic
//! similarity (synonyms, paraphrases), inject vectors from an external embedding
//! model via [`ChaoticSemanticFramework::inject_concept`].
//!
//! **Turso Vector Alternative**: This crate uses libSQL (local SQLite or remote Turso)
//! for persistence. You can add Turso's native vector search tables (`F32_BLOB` with
//! `vector_top_k()`) alongside the crate's HDC storage in the same database. The crate
//! manages `concepts` and `associations` tables; you manage `semantic_vectors` for
//! float-vector similarity search.
//!
//! ## Concurrency
//!
//! The framework uses `tokio::sync::RwLock` internally and is safe to share
//! across async tasks via `Arc`. SQLite persistence uses WAL mode for concurrent
//! reads. All public APIs are async — do not wrap in `block_on` inside a Tokio runtime.
//!
//! ## WASM Parity
//!
//! Most APIs are available in WASM. Exceptions:
//! - `Persistence`: Replaced with stubs in `persistence_wasm`
//! - `process_sequence`: No Rayon parallelization in WASM

pub use bridge_retrieval::BridgeRetrieval;
pub use bundle::BundleAccumulator;
pub use error::{MemoryError, Result};
pub use framework::ChaoticSemanticFramework;
pub use framework_builder::FrameworkBuilder;
pub use framework_events::MemoryEvent;
pub use hyperdim::{HVec10240, batch_cosine_similarity};
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
mod framework_graph_rag;
mod framework_metrics;
#[cfg(not(target_arch = "wasm32"))]
mod framework_ops;
mod framework_ttl;
mod framework_validation;
pub mod graph_traversal;
pub mod hyperdim;
mod hyperdim_batch;
mod hyperdim_simd; // AVX2/NEON SIMD paths
#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
pub mod mcp;
pub mod metadata_filter;
pub mod semantic_triples;
pub use metadata_filter::MetadataFilter;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub mod persistence;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_migrations;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_ops;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_versions;
#[cfg(target_arch = "wasm32")]
pub mod persistence_wasm;
pub mod reservoir;
mod reservoir_inertial; // ADR-0064
mod reservoir_sparse; // LOC gate extraction
pub mod retrieval;
pub mod semantic_bridge;
pub mod singularity;
mod singularity_cache;
mod singularity_ext;
mod singularity_retrieval;
mod singularity_ttl;

#[cfg(target_arch = "wasm32")]
pub use crate::persistence_wasm as persistence;

// Stub persistence module when persistence feature is disabled on non-WASM
#[cfg(all(not(target_arch = "wasm32"), not(feature = "persistence")))]
pub mod persistence {
    //! Stub persistence module when the "persistence" feature is disabled.
    //! Enable the "persistence" feature for full libSQL-backed persistence.

    use crate::error::Result;
    use crate::hyperdim::HVec10240;
    use crate::singularity::Concept;

    /// Stub persistence type when persistence feature is disabled.
    #[derive(Debug)]
    pub struct Persistence;

    /// Stub concept version type when persistence feature is disabled.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ConceptVersion {
        pub concept_id: String,
        pub version: i64,
        pub vector: HVec10240,
        pub metadata: serde_json::Value,
        pub modified_at: u64,
    }

    impl Persistence {
        pub async fn save_concept(&self, _concept: &Concept) -> Result<()> {
            Ok(())
        }

        pub async fn save_concepts(&self, _concepts: &[Concept]) -> Result<()> {
            Ok(())
        }

        pub async fn load_concept(&self, _id: &str) -> Result<Option<Concept>> {
            Ok(None)
        }

        pub async fn load_all_concepts(&self) -> Result<Vec<Concept>> {
            Ok(Vec::new())
        }

        pub async fn delete_concept(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        pub async fn save_association(&self, _from: &str, _to: &str, _strength: f32) -> Result<()> {
            Ok(())
        }

        pub async fn save_associations(
            &self,
            _associations: &[(String, String, f32)],
        ) -> Result<()> {
            Ok(())
        }

        pub async fn load_associations(&self, _id: &str) -> Result<Vec<(String, f32)>> {
            Ok(Vec::new())
        }

        pub async fn delete_association(&self, _from: &str, _to: &str) -> Result<()> {
            Ok(())
        }

        pub async fn clear_concept_associations(&self, _id: &str) -> Result<()> {
            Ok(())
        }

        pub async fn clear_all(&self) -> Result<()> {
            Ok(())
        }

        pub async fn checkpoint(&self) -> Result<()> {
            Ok(())
        }

        pub async fn health_check(&self) -> Result<()> {
            Ok(())
        }

        pub async fn size(&self) -> Result<u64> {
            Ok(0)
        }

        pub async fn backup(&self, _path: &str) -> Result<()> {
            Ok(())
        }

        pub async fn restore(&self, _path: &str) -> Result<()> {
            Ok(())
        }

        pub async fn get_concept_history(
            &self,
            _id: &str,
            _limit: usize,
        ) -> Result<Vec<ConceptVersion>> {
            Ok(Vec::new())
        }

        pub async fn schema_version(&self) -> Result<i64> {
            Ok(0)
        }

        pub async fn apply_migrations(&self, _target_version: i64) -> Result<()> {
            Ok(())
        }
    }
}

pub mod prelude {
    pub use crate::bridge_retrieval::BridgeRetrieval;
    pub use crate::bundle::BundleAccumulator;
    pub use crate::error::{MemoryError, Result};
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::framework_events::MemoryEvent;
    pub use crate::hyperdim::HVec10240;
    pub use crate::semantic_bridge::{BridgeHit, ConceptGraph, MemoryPacket};
    pub use crate::singularity::{Concept, ConceptBuilder};
    pub use crate::singularity_retrieval::{
        CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats,
    };
}

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_ext;
#[cfg(target_arch = "wasm32")]
mod wasm_graph_rag;
// Include wasm_ext for tests to run underlying data pattern tests on native
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wasm_ext;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wasm_graph_rag;
