//! WASM persistence stubs.
//!
//! Persistence is unavailable on `wasm32` in this crate build.
//! All operations return `MemoryError::UnsupportedOperation`.

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
    /// Format of the stored vector (e.g., "float", "binary")
    pub vector_format: String,
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

    pub async fn save_association(
        &self,
        _ns: &str,
        _from: &str,
        _to: &str,
        _strength: f32,
    ) -> Result<()> {
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

    pub async fn clear_namespace(&self, _ns: &str) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn clear_all(&self) -> Result<()> {
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

    pub async fn save_index(&self, _ns: &str, _id: &str, _data: &[u8]) -> Result<()> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        Err(wasm_persistence_unavailable())
    }

    pub async fn load_index(&self, _ns: &str, _id: &str) -> Result<Option<Vec<u8>>> {
        Err(wasm_persistence_unavailable())
    }
}

fn wasm_persistence_unavailable() -> MemoryError {
    MemoryError::UnsupportedOperation("Persistence is unavailable on wasm32".to_string())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_local_returns_unsupported() {
        let result = Persistence::new_local("test.db").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));
    }

    #[tokio::test]
    async fn new_turso_returns_unsupported() {
        let result = Persistence::new_turso("https://example.com", "token").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));
    }

    #[tokio::test]
    async fn all_operations_return_unsupported() {
        // Persistence struct cannot be created (constructor fails),
        // but we can verify the error function behavior
        let err = wasm_persistence_unavailable();

        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));

        // Verify error message contains expected text
        let msg = err.to_string();
        assert!(msg.contains("wasm32"));
        assert!(msg.contains("unavailable"));
    }

    #[test]
    fn concept_version_serialization_roundtrip() {
        // ConceptVersion is used in WASM builds for version serialization
        let version = ConceptVersion {
            concept_id: "test-id".to_string(),
            version: 1,
            vector: HVec10240::zero(),
            metadata: serde_json::json!({"key": "value"}),
            modified_at: 12345,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&version).unwrap();

        // Deserialize back
        let recovered: ConceptVersion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered.concept_id, version.concept_id);
        assert_eq!(recovered.version, version.version);
        assert_eq!(recovered.modified_at, version.modified_at);
    }
}
