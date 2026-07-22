use crate::export_payload::{BinaryExportPayload, ExportPayload, unix_now_secs};
use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

impl ChaoticSemanticFramework {
    /// Set the current namespace.
    pub async fn set_namespace(&self, ns: impl Into<String>) -> Result<()> {
        let ns = ns.into();
        Self::validate_namespace(&ns)?;
        *self.namespace.write().await = ns;
        Ok(())
    }

    /// List all namespaces, querying persistence if available for complete results.
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let mut namespaces: Vec<String> = {
            let sing = self.singularity.read().await;
            sing.namespaces.keys().cloned().collect()
        };

        // Query persistence for namespaces not yet loaded in memory
        if let Some(ref persistence) = self.persistence {
            if let Ok(persisted) = persistence.list_namespaces().await {
                for ns in persisted {
                    if !namespaces.contains(&ns) {
                        namespaces.push(ns);
                    }
                }
            }
        }

        Ok(namespaces)
    }

    /// Delete a namespace: remove from memory first, then persist.
    ///
    /// Memory-first order is intentional: if the persistence deletion fails,
    /// the data still exists in the database and will be reloaded on the next
    /// framework access (no data loss). The reverse order (persist-then-memory)
    /// could leave data in memory that no longer has a persistence backing,
    /// causing inconsistency on process restart.
    pub async fn delete_namespace(&self, ns: &str) -> Result<usize> {
        Self::validate_namespace(ns)?;
        let count = {
            let mut sing = self.singularity.write().await;
            let count = sing.len(ns);
            sing.namespaces.remove(ns);
            count
        };

        // Persist after memory removal; if DB fails, data still exists in DB
        // and will be reloaded on next access (no data loss).
        if let Some(ref persistence) = self.persistence {
            persistence.clear_namespace(ns).await?;
        }

        Ok(count)
    }

    /// Export a namespace to a JSON file.
    ///
    /// Loads the namespace data from persistence if not currently in memory,
    /// then exports using the existing `export_json` logic.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn export_namespace(&self, ns: &str, path: &Path) -> Result<()> {
        Self::validate_namespace(ns)?;
        let path_str =
            path.to_str()
                .ok_or_else(|| csm_core_lib::error::MemoryError::InvalidInput {
                    field: "path".to_string(),
                    reason: "Invalid path".to_string(),
                })?;

        // Ensure the namespace is loaded from persistence if available and not in memory
        self.ensure_namespace_loaded(ns).await?;

        // Use a temporary framework scoped to the target namespace for export
        let temp_fw = self.clone_with_namespace(ns);
        temp_fw.export_json(path_str).await
    }

    /// Export a namespace to bytes (in-memory, WASM-compatible).
    ///
    /// Loads the namespace data from persistence if not currently in memory,
    /// then serializes to a binary payload using bincode.
    pub async fn export_namespace_to_bytes(&self, ns: &str) -> Result<Vec<u8>> {
        Self::validate_namespace(ns)?;
        // Ensure the namespace is loaded from persistence if available and not in memory
        self.ensure_namespace_loaded(ns).await?;

        // Build the export payload scoped to the target namespace
        let payload = {
            let sing = self.singularity.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(ns),
                associations: sing.all_associations(ns),
            }
        };

        let binary_payload = BinaryExportPayload::from(payload);
        let data = bincode::serialize(&binary_payload).map_err(|e| {
            csm_core_lib::error::MemoryError::Persistence(format!("Serialization error: {e}"))
        })?;
        Ok(data)
    }

    /// Load namespace data into memory from persistence if not already loaded.
    ///
    /// Uses an atomic check-and-load pattern to avoid a TOCTOU race where two
    /// concurrent calls both see the namespace as absent and attempt to load it.
    /// The re-check under the write lock ensures only one caller performs the
    /// injection. Persistence errors are propagated so callers do not silently
    /// receive empty/incomplete exports.
    async fn ensure_namespace_loaded(&self, ns: &str) -> Result<()> {
        {
            let sing = self.singularity.read().await;
            if sing.namespaces.contains_key(ns) {
                return Ok(());
            }
        }

        if let Some(ref persistence) = self.persistence {
            let concepts = persistence.load_all_concepts(ns).await?;

            let mut sing = self.singularity.write().await;
            // Re-check under write lock (TOCTOU guard)
            if !sing.namespaces.contains_key(ns) {
                for concept in concepts {
                    sing.inject(ns, concept)?;
                }
            }
        }

        Ok(())
    }

    fn clone_with_namespace(&self, ns: &str) -> Self {
        Self {
            singularity: self.singularity.clone(),
            persistence: self.persistence.clone(),
            reservoir: self.reservoir.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            event_sender: self.event_sender.clone(),
            emitters: self.emitters.clone(),
            namespace: Arc::new(RwLock::new(ns.to_string())),
            embedding_provider: self.embedding_provider.clone(),
            projection: self.projection.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::export_payload::BinaryExportPayload;
    use csm_core_lib::hyperdim::HVec10240;
    async fn empty_framework() -> ChaoticSemanticFramework {
        ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .expect("framework build should succeed")
    }

    #[tokio::test]
    async fn test_set_namespace_updates_state() {
        let fw = empty_framework().await;
        assert_eq!(fw.namespace().await, "_default");
        fw.set_namespace("new-ns").await.unwrap();
        assert_eq!(fw.namespace().await, "new-ns");

        // Verify validation works and preserves state on error
        assert!(fw.set_namespace("").await.is_err());
        assert_eq!(fw.namespace().await, "new-ns");
    }

    #[tokio::test]
    async fn test_export_namespace_to_bytes_serializes_concepts() {
        let fw = empty_framework().await;
        fw.set_namespace("test-ns").await.unwrap();
        let vector = HVec10240::random();

        fw.inject_concept("c1", vector).await.unwrap();
        fw.inject_concept("c2", HVec10240::random()).await.unwrap();
        fw.associate("c1", "c2", 0.5_f32).await.unwrap();

        let bytes = fw.export_namespace_to_bytes("test-ns").await.unwrap();
        assert!(!bytes.is_empty(), "export bytes should not be empty");

        let bin_payload: BinaryExportPayload =
            bincode::deserialize(&bytes).expect("should deserialize bincode payload");
        assert_eq!(bin_payload.concepts.len(), 2, "should have 2 concepts");
        assert_eq!(
            bin_payload.associations.len(),
            1,
            "should have 1 association"
        );
        assert_eq!(
            bin_payload.associations[0],
            ("c1".to_string(), "c2".to_string(), 0.5_f32)
        );

        let c1 = bin_payload
            .concepts
            .iter()
            .find(|c| c.id == "c1")
            .expect("c1 should exist in export");
        let restored = HVec10240::from_bytes(&c1.vector_bytes).unwrap();
        assert_eq!(
            restored.to_bytes(),
            vector.to_bytes(),
            "vector should survive roundtrip"
        );
    }

    #[tokio::test]
    async fn test_export_namespace_to_bytes_empty_namespace() {
        let fw = empty_framework().await;

        let bytes = fw.export_namespace_to_bytes("empty-ns").await.unwrap();
        let bin_payload: BinaryExportPayload =
            bincode::deserialize(&bytes).expect("should deserialize bincode payload");
        assert!(
            bin_payload.concepts.is_empty(),
            "empty namespace should have no concepts"
        );
        assert!(
            bin_payload.associations.is_empty(),
            "empty namespace should have no associations"
        );
    }

    #[tokio::test]
    async fn test_export_namespace_to_bytes_namespace_not_found() {
        let fw = empty_framework().await;

        let bytes = fw.export_namespace_to_bytes("nonexistent").await.unwrap();
        let bin_payload: BinaryExportPayload =
            bincode::deserialize(&bytes).expect("should deserialize bincode payload");
        assert!(bin_payload.concepts.is_empty());
    }

    #[tokio::test]
    async fn test_delete_namespace_returns_correct_count() {
        let fw = empty_framework().await;
        fw.set_namespace("delete-test").await.unwrap();
        fw.inject_concept("c1", HVec10240::random()).await.unwrap();
        fw.inject_concept("c2", HVec10240::random()).await.unwrap();

        // Should return 2
        let count = fw.delete_namespace("delete-test").await.unwrap();
        assert_eq!(
            count, 2,
            "delete_namespace should return the number of deleted concepts"
        );

        // Verify it's actually gone from list_namespaces
        let namespaces = fw.list_namespaces().await.unwrap();
        assert!(
            !namespaces.contains(&"delete-test".to_string()),
            "namespace should be removed from list"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_export_namespace_writes_file() {
        let fw = empty_framework().await;
        fw.set_namespace("export-test").await.unwrap();
        fw.inject_concept("c1", HVec10240::random()).await.unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("export.json");

        fw.export_namespace("export-test", &path).await.unwrap();

        assert!(path.exists(), "export file should exist");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("c1"),
            "export file should contain concept id"
        );
        assert!(content.len() > 100, "export file should not be empty");
    }

    #[tokio::test]
    async fn test_export_namespace_to_bytes_isolates_namespaces() {
        let fw = empty_framework().await;
        let vector_a = HVec10240::random();
        let vector_b = HVec10240::random();

        fw.set_namespace("ns-a").await.unwrap();
        fw.inject_concept("a1", vector_a).await.unwrap();

        fw.set_namespace("ns-b").await.unwrap();
        fw.inject_concept("b1", vector_b).await.unwrap();

        let bytes = fw.export_namespace_to_bytes("ns-a").await.unwrap();
        let bin_payload: BinaryExportPayload =
            bincode::deserialize(&bytes).expect("should deserialize");
        assert_eq!(bin_payload.concepts.len(), 1, "ns-a should have 1 concept");
        assert_eq!(
            bin_payload.concepts[0].id, "a1",
            "ns-a should contain a1 only"
        );
    }

    #[tokio::test]
    async fn test_delete_namespace_logic() {
        let fw = empty_framework().await;
        fw.set_namespace("ns-delete").await.unwrap();
        fw.inject_concept("c1", HVec10240::random()).await.unwrap();
        fw.inject_concept("c2", HVec10240::random()).await.unwrap();

        let count = fw.delete_namespace("ns-delete").await.unwrap();
        assert_eq!(count, 2, "should return correct deleted count");

        let namespaces = fw.list_namespaces().await.unwrap();
        assert!(
            !namespaces.contains(&"ns-delete".to_string()),
            "namespace should be removed"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_export_namespace_file() {
        // Use /tmp since validate_path requires it or current dir
        let export_path = std::path::PathBuf::from("/tmp/export_ns_test.json");
        if export_path.exists() {
            let _ = std::fs::remove_file(&export_path);
        }

        let fw = empty_framework().await;
        fw.set_namespace("ns-export").await.unwrap();
        fw.inject_concept("c1", HVec10240::random()).await.unwrap();

        fw.export_namespace("ns-export", &export_path)
            .await
            .unwrap();
        assert!(export_path.exists(), "export file should exist");

        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(
            content.contains("\"id\": \"c1\""),
            "export should contain concept"
        );
        let _ = std::fs::remove_file(&export_path);
    }

    #[tokio::test]
    async fn test_validate_namespace_integration() {
        let fw = empty_framework().await;
        // Test set_namespace validation
        assert!(fw.set_namespace("").await.is_err());
        assert!(fw.set_namespace("a".repeat(129)).await.is_err());
        assert!(fw.set_namespace("ns\0").await.is_err());

        // Test delete_namespace validation
        assert!(fw.delete_namespace("").await.is_err());

        // Test export validation
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = std::path::Path::new("test.json");
            assert!(fw.export_namespace("", path).await.is_err());
        }
        assert!(fw.export_namespace_to_bytes("").await.is_err());
    }
}
