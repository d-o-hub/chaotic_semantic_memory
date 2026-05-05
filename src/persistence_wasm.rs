//! WASM stub for persistence

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConceptVersion {
    pub concept_id: String,
    pub version: i64,
    pub vector: HVec10240,
    pub metadata: serde_json::Value,
    pub modified_at: u64,
}

#[derive(Debug)]
pub struct Persistence;

impl Persistence {
    pub async fn save_concept(&self, _ns: &str, _concept: &Concept) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_concepts(&self, _ns: &str, _concepts: &[Concept]) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_concept(&self, _ns: &str, _id: &str) -> Result<Option<Concept>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_all_concepts(&self, _ns: &str) -> Result<Vec<Concept>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn delete_concept(&self, _ns: &str, _id: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_association(&self, _ns: &str, _from: &str, _to: &str, _strength: f32) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_associations(
        &self,
        _ns: &str,
        _associations: &[(String, String, f32)],
    ) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_associations(&self, _ns: &str, _id: &str) -> Result<Vec<(String, f32)>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn delete_association(&self, _ns: &str, _from: &str, _to: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn clear_concept_associations(&self, _ns: &str, _id: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn clear_namespace(&self, _ns: &str) -> Result<()> {
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

    pub async fn backup(&self, _path: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn restore(&self, _path: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn get_concept_history(
        &self,
        _ns: &str,
        _id: &str,
        _limit: usize,
    ) -> Result<Vec<ConceptVersion>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn schema_version(&self) -> Result<i64> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn save_index(&self, _ns: &str, _id: &str, _data: &[u8]) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_index(&self, _ns: &str, _id: &str) -> Result<Option<Vec<u8>>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn apply_migrations(&self, _target_version: i64) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_all_canonical_concepts(&self) -> Result<Vec<crate::semantic_bridge::CanonicalConcept>> {
        Err(wasm_persistence_unavailable())
    }
}

fn wasm_persistence_unavailable() -> MemoryError {
    MemoryError::Unsupported {
        operation: "persistence".to_string(),
        reason: "libSQL persistence not available in WASM target".to_string(),
    }
}
