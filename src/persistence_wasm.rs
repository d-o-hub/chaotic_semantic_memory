//! Persistence stub for wasm32 builds.
//!
//! Canonical stub lives in `csm-persistence` (ADR-0094); this module keeps the
//! root public path (`chaotic_semantic_memory::persistence_wasm`, re-exported as
//! `persistence` on wasm32) and re-exports the owner's type.

pub use csm_memory::ConceptVersion;
pub use csm_persistence::Persistence;
