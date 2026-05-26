use crate::error::Result;
use crate::export_payload::{BinaryExportPayload, unix_now_secs};
use crate::framework::ChaoticSemanticFramework;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

impl ChaoticSemanticFramework {
    /// Set the current namespace.
    pub async fn set_namespace(&self, ns: impl Into<String>) {
        let mut namespace = self.namespace.write().await;
        *namespace = ns.into();
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
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::error::MemoryError::InvalidInput {
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
        // Ensure the namespace is loaded from persistence if available and not in memory
        self.ensure_namespace_loaded(ns).await?;

        // Build the export payload scoped to the target namespace
        let payload = {
            let sing = self.singularity.read().await;
            let concepts = sing.all_concepts(ns);
            let associations = sing.all_associations(ns);
            drop(sing);
            crate::export_payload::ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts,
                associations,
            }
        };

        let binary_payload = BinaryExportPayload::from(payload);
        let data = bincode::serialize(&binary_payload).map_err(|e| {
            crate::error::MemoryError::Persistence(format!("Serialization error: {}", e))
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
    use super::*;
    use crate::export_payload::BinaryExportPayload;
    use crate::hyperdim::HVec10240;
    async fn empty_framework() -> ChaoticSemanticFramework {
        ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .expect("framework build should succeed")
    }

    #[tokio::test]
    async fn test_export_namespace_to_bytes_serializes_concepts() {
        let fw = empty_framework().await;
        fw.set_namespace("test-ns").await;
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
    async fn test_export_namespace_to_bytes_isolates_namespaces() {
        let fw = empty_framework().await;
        let vector_a = HVec10240::random();
        let vector_b = HVec10240::random();

        fw.set_namespace("ns-a").await;
        fw.inject_concept("a1", vector_a).await.unwrap();

        fw.set_namespace("ns-b").await;
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
}
