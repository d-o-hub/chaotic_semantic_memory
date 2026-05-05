use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
use std::path::Path;

impl ChaoticSemanticFramework {
    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let sing = self.singularity.read().await;
        Ok(sing.namespaces.keys().cloned().collect())
    }

    pub async fn delete_namespace(&self, ns: &str) -> Result<usize> {
        // Persist deletion first so DB failure doesn't leave orphaned data
        if let Some(ref persistence) = self.persistence {
            persistence.clear_namespace(ns).await?;
        }

        let count = {
            let mut sing = self.singularity.write().await;
            let count = sing.len(ns);
            sing.namespaces.remove(ns);
            count
        };

        Ok(count)
    }

    pub async fn export_namespace(&self, ns: &str, path: &Path) -> Result<()> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::error::MemoryError::InvalidInput {
                field: "path".to_string(),
                reason: "Invalid path".to_string(),
            })?;

        // We temporarily switch the framework's namespace to export a specific one
        // using the existing export_json logic.
        let temp_fw = self.clone_with_namespace(ns);
        temp_fw.export_json(path_str).await
    }

    fn clone_with_namespace(&self, ns: &str) -> Self {
        Self {
            singularity: self.singularity.clone(),
            persistence: self.persistence.clone(),
            reservoir: self.reservoir.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            event_sender: self.event_sender.clone(),
            namespace: ns.to_string(),
        }
    }
}
