//! WASM persistence stubs.
//!
//! Persistence is unavailable on `wasm32` in this crate build.

use crate::error::{MemoryError, Result};
use crate::singularity::Concept;

/// Persistence stub for wasm32 builds.
pub struct Persistence;

impl Persistence {
    pub async fn new_local(_path: &str) -> Result<Self> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn new_turso(_url: &str, _token: &str) -> Result<Self> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_concept(&self, _concept: &Concept) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_concepts(&self, _concepts: &[Concept]) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_concept(&self, _id: &str) -> Result<Option<Concept>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_all_concepts(&self) -> Result<Vec<Concept>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn delete_concept(&self, _id: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_association(&self, _from: &str, _to: &str, _strength: f32) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_associations(&self, _associations: &[(String, String, f32)]) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_associations(&self, _id: &str) -> Result<Vec<(String, f32)>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn checkpoint(&self) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn size(&self) -> Result<u64> {
        Err(wasm_persistence_unavailable())
    }
}

fn wasm_persistence_unavailable() -> MemoryError {
    MemoryError::Persistence("Persistence is unavailable on wasm32".to_string())
}
