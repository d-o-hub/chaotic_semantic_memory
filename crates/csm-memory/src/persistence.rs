use csm_core::Result;
use crate::singularity::{Concept, ConceptVersion};

/// Persistence trait for the framework.
///
/// This trait defines the interface for persisting concepts, associations,
/// and other framework data.
#[async_trait::async_trait]
pub trait Persistence: Send + Sync + std::fmt::Debug {
    async fn save_concept(&self, ns: &str, concept: &Concept) -> Result<()>;
    async fn save_concepts(&self, ns: &str, concepts: &[Concept]) -> Result<()>;
    async fn load_concept(&self, ns: &str, id: &str) -> Result<Option<Concept>>;
    async fn load_all_concepts(&self, ns: &str) -> Result<Vec<Concept>>;
    async fn delete_concept(&self, ns: &str, id: &str) -> Result<()>;
    async fn save_association(&self, ns: &str, from: &str, to: &str, strength: f32) -> Result<()>;
    async fn save_associations(&self, ns: &str, associations: &[(String, String, f32)]) -> Result<()>;
    async fn load_associations(&self, ns: &str, id: &str) -> Result<Vec<(String, f32)>>;
    async fn delete_association(&self, ns: &str, from: &str, to: &str) -> Result<()>;
    async fn clear_concept_associations(&self, ns: &str, id: &str) -> Result<()>;
    async fn clear_all(&self) -> Result<()>;
    async fn checkpoint(&self) -> Result<()>;
    async fn health_check(&self) -> Result<()>;
    async fn size(&self) -> Result<u64>;
    async fn backup(&self, path: &str) -> Result<()>;
    async fn restore(&self, path: &str) -> Result<()>;
    async fn get_version_scoped(&self, ns: &str, id: &str, version: u64) -> Result<Option<Concept>>;
    async fn list_versions_scoped(&self, ns: &str, id: &str) -> Result<Vec<ConceptVersion>>;
    async fn get_concept_history(&self, ns: &str, id: &str, limit: usize) -> Result<Vec<ConceptVersion>>;
    async fn schema_version(&self) -> Result<i64>;
    async fn save_index(&self, ns: &str, id: &str, data: &[u8]) -> Result<()>;
    async fn load_index(&self, ns: &str, id: &str) -> Result<Option<Vec<u8>>>;
    async fn apply_migrations(&self, target_version: i64) -> Result<()>;
    async fn list_namespaces(&self) -> Result<Vec<String>>;
    async fn clear_namespace(&self, ns: &str) -> Result<()>;
}
