//! Stub persistence implementation for WASM environments.
//! Currently WASM environments do not support the full libSQL persistence layer.

use crate::persistence::Persistence;
use csm_core::Result;
use crate::singularity::{Concept, ConceptVersion};

#[derive(Debug, Default)]
pub struct WasmPersistence;

#[async_trait::async_trait]
impl Persistence for WasmPersistence {
    async fn save_concept(&self, _ns: &str, _concept: &Concept) -> Result<()> {
        Ok(())
    }

    async fn save_concepts(&self, _ns: &str, _concepts: &[Concept]) -> Result<()> {
        Ok(())
    }

    async fn load_concept(&self, _ns: &str, _id: &str) -> Result<Option<Concept>> {
        Ok(None)
    }

    async fn load_all_concepts(&self, _ns: &str) -> Result<Vec<Concept>> {
        Ok(Vec::new())
    }

    async fn delete_concept(&self, _ns: &str, _id: &str) -> Result<()> {
        Ok(())
    }

    async fn save_association(&self, _ns: &str, _from: &str, _to: &str, _strength: f32) -> Result<()> {
        Ok(())
    }

    async fn save_associations(&self, _ns: &str, _associations: &[(String, String, f32)]) -> Result<()> {
        Ok(())
    }

    async fn load_associations(&self, _ns: &str, _id: &str) -> Result<Vec<(String, f32)>> {
        Ok(Vec::new())
    }

    async fn delete_association(&self, _ns: &str, _from: &str, _to: &str) -> Result<()> {
        Ok(())
    }

    async fn clear_concept_associations(&self, _ns: &str, _id: &str) -> Result<()> {
        Ok(())
    }

    async fn clear_all(&self) -> Result<()> {
        Ok(())
    }

    async fn checkpoint(&self) -> Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }

    async fn size(&self) -> Result<u64> {
        Ok(0)
    }

    async fn backup(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    async fn restore(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    async fn get_version_scoped(&self, _ns: &str, _id: &str, _version: u64) -> Result<Option<Concept>> {
        Ok(None)
    }

    async fn list_versions_scoped(&self, _ns: &str, _id: &str) -> Result<Vec<ConceptVersion>> {
        Ok(Vec::new())
    }

    async fn get_concept_history(&self, _ns: &str, _id: &str, _limit: usize) -> Result<Vec<ConceptVersion>> {
        Ok(Vec::new())
    }

    async fn schema_version(&self) -> Result<i64> {
        Ok(0)
    }

    async fn save_index(&self, _ns: &str, _id: &str, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    async fn load_index(&self, _ns: &str, _id: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn apply_migrations(&self, _target_version: i64) -> Result<()> {
        Ok(())
    }

    async fn list_namespaces(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn clear_namespace(&self, _ns: &str) -> Result<()> {
        Ok(())
    }
}
