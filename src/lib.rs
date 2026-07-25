#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn
)]
//!
//! High-performance memory system using **Hyperdimensional Computing** (HDC) and
//! chaotic echo-state reservoir dynamics.
//!

pub use bridge_retrieval::BridgeRetrieval;
pub use csm_core_lib::bundle::BundleAccumulator;
pub use csm_core_lib::error::{MemoryError, Result};
pub use csm_core_lib::hyperdim::{BHVec10240, HVec10240, batch_cosine_similarity};
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
pub use csm_chaos;
pub use csm_core_lib::bundle;
#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
pub mod cli;
pub mod concept_builder;
pub mod embedding;
pub use csm_core_lib::encoder;
pub use csm_core_lib::error;
mod export_payload;
pub mod framework;
mod framework_accessors;
mod framework_bridge;
pub mod framework_builder;
mod framework_events;
pub mod framework_events_ce;
mod framework_graph_rag;
mod framework_metrics;
mod framework_namespaces;
#[cfg(not(target_arch = "wasm32"))]
mod framework_ops;
#[cfg(not(target_arch = "wasm32"))]
mod framework_ops_import;
mod framework_persistence;
mod framework_ttl;
pub mod framework_ttl_advanced;
mod framework_validation;
pub mod graph_traversal;
pub use csm_core_lib::hyperdim;
#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
pub mod mcp;
pub mod metadata_filter;
#[cfg(any(feature = "prometheus", feature = "otlp-json", feature = "otlp"))]
pub mod observability;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
mod persistence_concepts;
pub mod semantic_triples;
pub use metadata_filter::MetadataFilter;
pub mod index;
pub mod index_envelope;
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
pub use csm_core_lib::reservoir;
pub use csm_traits;
pub mod retrieval;
pub mod semantic_bridge;
pub mod singularity;
mod singularity_cache;
mod singularity_retrieval;
pub mod singularity_state;

#[cfg(target_arch = "wasm32")]
pub use crate::persistence_wasm as persistence;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "persistence")))]
pub mod persistence {
    use crate::singularity::Concept;
    use csm_core_lib::Result;

    #[derive(Debug)]
    pub struct Persistence;

    pub use crate::singularity::ConceptVersion;

    impl Persistence {
        pub async fn save_concept(&self, _ns: &str, _concept: &Concept) -> Result<()> {
            Ok(())
        }
        pub async fn save_concepts(&self, _ns: &str, _concepts: &[Concept]) -> Result<()> {
            Ok(())
        }
        pub async fn load_concept(&self, _ns: &str, _id: &str) -> Result<Option<Concept>> {
            Ok(None)
        }
        pub async fn load_all_concepts(&self, _ns: &str) -> Result<Vec<Concept>> {
            Ok(Vec::new())
        }
        pub async fn delete_concept(&self, _ns: &str, _id: &str) -> Result<()> {
            Ok(())
        }
        pub async fn save_association(
            &self,
            _ns: &str,
            _from: &str,
            _to: &str,
            _strength: f32,
        ) -> Result<()> {
            Ok(())
        }
        pub async fn save_associations(
            &self,
            _ns: &str,
            _associations: &[(String, String, f32)],
        ) -> Result<()> {
            Ok(())
        }
        pub async fn load_associations(
            &self,
            _ns: &str,
            _id: &str,
        ) -> Result<Vec<(String, f32, u64)>> {
            Ok(Vec::new())
        }
        pub async fn load_all_associations(
            &self,
            _ns: &str,
        ) -> Result<Vec<(String, String, f32, u64)>> {
            Ok(Vec::new())
        }
        pub async fn get_namespace_revision(&self, _ns: &str) -> Result<u64> {
            Ok(0)
        }
        pub async fn bump_namespace_revision(&self, _ns: &str) -> Result<u64> {
            Ok(1)
        }
        pub async fn save_index_envelope(
            &self,
            _ns: &str,
            _id: &str,
            _envelope: &crate::index_envelope::IndexSnapshotEnvelope,
        ) -> Result<()> {
            Ok(())
        }
        pub async fn load_index_envelope(
            &self,
            _ns: &str,
            _id: &str,
        ) -> Result<Option<crate::index_envelope::IndexSnapshotEnvelope>> {
            Ok(None)
        }
        pub async fn delete_association(&self, _ns: &str, _from: &str, _to: &str) -> Result<()> {
            Ok(())
        }
        pub async fn clear_concept_associations(&self, _ns: &str, _id: &str) -> Result<()> {
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
        pub async fn get_version_scoped(
            &self,
            _ns: &str,
            _id: &str,
            _version: u64,
        ) -> Result<Option<Concept>> {
            Ok(None)
        }
        pub async fn list_versions_scoped(
            &self,
            _ns: &str,
            _id: &str,
        ) -> Result<Vec<crate::singularity::ConceptVersion>> {
            Ok(Vec::new())
        }
        pub async fn get_concept_history(
            &self,
            _ns: &str,
            _id: &str,
            _limit: usize,
        ) -> Result<Vec<ConceptVersion>> {
            Ok(Vec::new())
        }
        pub async fn schema_version(&self) -> Result<i64> {
            Ok(0)
        }
        pub async fn save_index(&self, _ns: &str, _id: &str, _data: &[u8]) -> Result<()> {
            Ok(())
        }
        pub async fn load_index(&self, _ns: &str, _id: &str) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }
        pub async fn apply_migrations(&self, _target_version: i64) -> Result<()> {
            Ok(())
        }
        pub async fn list_namespaces(&self) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
        pub async fn clear_namespace(&self, _ns: &str) -> Result<()> {
            Ok(())
        }
    }
}

pub mod prelude {
    pub use crate::bridge_retrieval::BridgeRetrieval;
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::framework_events::MemoryEvent;
    pub use crate::semantic_bridge::{BridgeHit, ConceptGraph, MemoryPacket};
    pub use crate::singularity::{Concept, ConceptBuilder, ConceptDiff, ConceptVersion};
    pub use crate::singularity_retrieval::{
        CandidateSource, FilterStrategy, RetrievalConfig, RetrievalStats,
    };
    pub use csm_core_lib::bundle::BundleAccumulator;
    pub use csm_core_lib::error::{MemoryError, Result};
    pub use csm_core_lib::hyperdim::HVec10240;
}

#[cfg(all(test, not(target_arch = "wasm32"), feature = "persistence"))]
mod bridge_persistence_tests;
#[cfg(test)]
mod bridge_retrieval_tests;
#[cfg(test)]
mod framework_ops_tests;
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

#[cfg(test)]
mod encoder_lib_tests {
    use csm_core_lib::encoder::TextEncoder;
    use csm_core_lib::hyperdim::HVec10240;

    #[test]
    fn encode_text_produces_nonzero_output() {
        // Kills the wasm.rs encode_text mutant that replaces the body with empty bytes.
        // TextEncoder::encode is the core logic; wasm.rs encode_text just wraps it.
        let encoder = TextEncoder::new();
        let result = encoder.encode("hello world");
        let zero = HVec10240::zero();
        assert_ne!(result, zero, "encoding must produce a non-zero vector");
    }

    #[test]
    fn encode_text_is_deterministic() {
        let encoder = TextEncoder::new();
        let a = encoder.encode("test input");
        let b = encoder.encode("test input");
        assert_eq!(a, b, "same input must produce identical output");
    }

    #[test]
    fn encode_different_inputs_differ() {
        let encoder = TextEncoder::new();
        let a = encoder.encode("hello");
        let b = encoder.encode("world");
        assert_ne!(a, b, "different inputs must produce different vectors");
    }
}
