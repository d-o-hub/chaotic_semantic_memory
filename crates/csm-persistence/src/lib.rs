//! Persistence backends for chaotic_semantic_memory.
//!
//! This crate provides:
//! - libSQL/Turso storage backend
//! - In-memory persistence for WASM
//! - Schema migrations and versioning

#[cfg(feature = "persistence")]
mod persistence;
#[cfg(feature = "persistence")]
mod persistence_concepts;
#[cfg(feature = "persistence")]
mod persistence_index;
#[cfg(feature = "persistence")]
mod persistence_migrations;
#[cfg(feature = "persistence")]
mod persistence_ops;
#[cfg(feature = "persistence")]
mod persistence_versions;

#[cfg(feature = "wasm")]
mod persistence_wasm;

#[cfg(feature = "persistence")]
pub use persistence::Persistence;
