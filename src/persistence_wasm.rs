//! WASM persistence stubs.
//!
//! Persistence is unavailable on `wasm32` in this crate build.

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;

/// Persistence stub for wasm32 builds.
pub struct Persistence;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConceptVersion {
    pub concept_id: String,
    pub version: i64,
    pub vector: HVec10240,
    pub metadata: serde_json::Value,
    pub modified_at: u64,
}

impl Persistence {
    pub async fn new_local(_path: &str) -> Result<Self> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn new_local_with_retention(_path: &str, _retention: usize) -> Result<Self> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn new_turso(_url: &str, _token: &str) -> Result<Self> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn new_turso_with_pool(_url: &str, _token: &str, _pool_size: usize) -> Result<Self> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn new_turso_with_pool_and_retention(
        _url: &str,
        _token: &str,
        _pool_size: usize,
        _retention: usize,
    ) -> Result<Self> {
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

    pub async fn clear_all(&self) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn get_concept_history(
        &self,
        _id: &str,
        _limit: usize,
    ) -> Result<Vec<ConceptVersion>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn schema_version(&self) -> Result<i64> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn apply_migrations(&self, _target_version: i64) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn backup(&self, _path: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn restore(&self, _path: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn checkpoint(&self) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn health_check(&self) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn size(&self) -> Result<u64> {
        Err(wasm_persistence_unavailable())
    }
}

fn wasm_persistence_unavailable() -> MemoryError {
    MemoryError::UnsupportedOperation("Persistence is unavailable on wasm32".to_string())
}
