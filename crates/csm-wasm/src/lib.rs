//! WASM bindings for chaotic_semantic_memory.
//!
//! This crate provides JavaScript/WASM bindings for the core memory engine.
//! It re-exports WASM-compatible types from the main crate and extracted crates.

// Re-export WASM bindings from main crate.
// The main crate only exposes `wasm` on wasm32 targets (see src/lib.rs cfg gate),
// so this re-export must carry the same gate for host builds (clippy/test).
#[cfg(target_arch = "wasm32")]
pub use chaotic_semantic_memory::wasm::*;

// Re-export useful types from extracted crates
pub use csm_core_lib::error::{MemoryError, Result};
pub use csm_core_lib::hyperdim::HVec10240;
pub use csm_memory::{Concept, MetadataFilter, TraversalConfig};
pub use csm_retrieval::GraphRagConfig;
pub use csm_traits::{MAX_IMPORT_SIZE, MemoryEvent, unix_now_ns, unix_now_secs};
