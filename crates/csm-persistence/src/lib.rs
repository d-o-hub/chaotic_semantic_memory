//! Persistence backends for chaotic_semantic_memory.
//!
//! This crate provides:
//! - libSQL/Turso storage backend
//! - In-memory persistence for WASM
//! - Schema migrations and versioning

#[cfg(feature = "libsql")]
mod persistence;
mod persistence_concepts;
mod persistence_index;
mod persistence_migrations;
mod persistence_ops;
mod persistence_versions;

#[cfg(feature = "wasm")]
mod persistence_wasm;

#[cfg(feature = "libsql")]
pub use persistence::Persistence;
