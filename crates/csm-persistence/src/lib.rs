//! Persistence backends for chaotic_semantic_memory.
//!
//! This crate provides:
//! - libSQL/Turso storage backend
//! - In-memory persistence for WASM
//! - Schema migrations and versioning

#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_absence;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_bridge;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_concepts;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_index;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_migrations;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_ops;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_retry;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_versions;

#[cfg(feature = "wasm")]
mod persistence_wasm;

#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
pub use persistence::Persistence;

// The stub is the crate's `Persistence` only where the real implementation is
// not compiled (wasm32, or a host build without `persistence`). Without the
// `any(..)` guard, `--all-features` on a host target exports the name twice
// (E0252) — the two features are otherwise independent.
#[cfg(all(
    feature = "wasm",
    any(target_arch = "wasm32", not(feature = "persistence"))
))]
pub use persistence_wasm::Persistence;
