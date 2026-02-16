//! Chaotic Semantic Memory System

pub use error::{MemoryError, Result};
pub use framework::{ChaoticSemanticFramework, FrameworkBuilder};
pub use hyperdim::HVec10240;
pub use singularity::{Concept, ConceptBuilder};

pub mod error;
pub mod framework;
pub mod hyperdim;
pub mod persistence;
pub mod reservoir;
pub mod singularity;

pub mod prelude {
    pub use crate::error::{MemoryError, Result};
    pub use crate::framework::{ChaoticSemanticFramework, FrameworkBuilder};
    pub use crate::hyperdim::HVec10240;
    pub use crate::singularity::{Concept, ConceptBuilder};
}

#[cfg(target_arch = "wasm32")]
pub mod wasm;
