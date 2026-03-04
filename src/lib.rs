//! Chaotic Semantic Memory System.
//!
//! High-performance semantic memory using hyperdimensional computing and
//! chaotic reservoir dynamics.
//!
//! # WASM Parity
//! Most APIs available in WASM. Exceptions:
//! - `Persistence`: Replaced with stubs in `persistence_wasm`
//! - `process_sequence`: No Rayon parallelization in WASM

pub use error::{MemoryError, Result};
pub use framework::ChaoticSemanticFramework;
pub use framework_builder::FrameworkBuilder;
pub use hyperdim::{HVec10240, batch_cosine_similarity};
pub use singularity::{Concept, ConceptBuilder};

pub mod bundle;
#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
pub mod cli;
pub mod concept_builder;
pub mod encoder;
pub mod error;
mod export_payload;
pub mod framework;
pub mod framework_builder;
#[cfg(not(target_arch = "wasm32"))]
mod framework_ops;
mod framework_validation;
pub mod graph_traversal;
pub mod hyperdim;
pub mod metadata_filter;
#[cfg(not(target_arch = "wasm32"))]
pub mod persistence;
#[cfg(not(target_arch = "wasm32"))]
mod persistence_ops;
#[cfg(target_arch = "wasm32")]
pub mod persistence_wasm;
pub mod reservoir;
pub mod singularity;
mod singularity_ext;

#[cfg(target_arch = "wasm32")]
pub use crate::persistence_wasm as persistence;

pub mod prelude {
    pub use crate::error::{MemoryError, Result};
    pub use crate::framework::ChaoticSemanticFramework;
    pub use crate::framework_builder::FrameworkBuilder;
    pub use crate::hyperdim::HVec10240;
    pub use crate::singularity::{Concept, ConceptBuilder};
}

#[cfg(target_arch = "wasm32")]
pub mod wasm;
