//! Persistence backends for chaotic_semantic_memory.
//!
//! This crate provides:
//! - libSQL/Turso storage backend
//! - In-memory persistence for WASM
//! - Schema migrations and versioning

#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_concepts;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_index;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_migrations;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_ops;
#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
mod persistence_versions;

#[cfg(feature = "wasm")]
mod persistence_wasm;

#[cfg(all(feature = "persistence", not(target_arch = "wasm32")))]
pub use persistence::Persistence;
