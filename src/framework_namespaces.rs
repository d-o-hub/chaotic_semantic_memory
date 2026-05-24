use crate::error::Result;
use crate::export_payload::{BinaryExportPayload, ExportPayload, unix_now_secs};
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
        let needs_load = self.needs_namespace_load(ns).await;
        if needs_load {
            self.load_namespace_from_persistence(ns).await;
        }

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
        let needs_load = self.needs_namespace_load(ns).await;
        if needs_load {
            self.load_namespace_from_persistence(ns).await;
        }

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
            crate::error::MemoryError::Persistence(format!("Serialization error: {}", e))
        })?;
        Ok(data)
    }

    async fn needs_namespace_load(&self, ns: &str) -> bool {
        let sing = self.singularity.read().await;
        !sing.namespaces.contains_key(ns)
    }

    async fn load_namespace_from_persistence(&self, ns: &str) {
        if let Some(ref persistence) = self.persistence {
            if let Ok(concepts) = persistence.load_all_concepts(ns).await {
                let mut sing = self.singularity.write().await;
                for concept in concepts {
                    let _ = sing.inject(ns, concept);
                }
            }
        }
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
