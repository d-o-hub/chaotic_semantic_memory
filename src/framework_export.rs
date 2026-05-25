#![cfg(not(target_arch = "wasm32"))]
//! Framework export and import operations.
//!
//! Extracted from framework_ops.rs to satisfy the 500 LOC gate.

use crate::error::Result;
use crate::export_payload::{BinaryConcept, BinaryExportPayload, ExportPayload, unix_now_secs};
use crate::framework::{ChaoticSemanticFramework, MAX_IMPORT_SIZE};
use crate::framework_validation::validate_path;
use bincode::Options;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tracing::{instrument, warn};

impl ChaoticSemanticFramework {
    /// Export memory state to JSON file using streaming to avoid OOM.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_json(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let mut file = File::create(validated_path).await?;
        let mut writer = BufWriter::new(&mut file);

        let version = env!("CARGO_PKG_VERSION");
        let exported_at = unix_now_secs();

        // Write JSON header
        writer
            .write_all(
                format!(
                    "{{\"version\":\"{}\",\"exported_at\":{},\"concepts\":[",
                    version, exported_at
                )
                .as_bytes(),
            )
            .await?;

        let ns = self.namespace.read().await.clone();
        let mut first = true;

        if let Some(ref persistence) = self.persistence {
            let (tx, mut rx) = mpsc::channel(32);
            let persistence_clone = persistence.clone();
            let ns_clone = ns.clone();

            tokio::spawn(async move {
                let _ = persistence_clone
                    .for_each_concept_scoped(&ns_clone, |concept| {
                        let tx = tx.clone();
                        async move {
                            let _ = tx.send(concept).await;
                            Ok(())
                        }
                    })
                    .await;
            });

            while let Some(concept) = rx.recv().await {
                if !first {
                    writer.write_all(b",").await?;
                }
                first = false;
                let data = serde_json::to_vec(&concept)?;
                writer.write_all(&data).await?;
            }
        } else {
            let sing = self.singularity.read().await;
            if let Some(ns_state) = sing.get_namespace(&ns) {
                for concept in ns_state.concepts.values() {
                    if !first {
                        writer.write_all(b",").await?;
                    }
                    first = false;
                    let data = serde_json::to_vec(concept)?;
                    writer.write_all(&data).await?;
                }
            }
        }

        writer.write_all(b"],\"associations\":[").await?;
        first = true;

        if let Some(ref persistence) = self.persistence {
            let (tx, mut rx) = mpsc::channel(32);
            let persistence_clone = persistence.clone();
            let ns_clone = ns.clone();

            tokio::spawn(async move {
                let _ = persistence_clone
                    .for_each_association_scoped(&ns_clone, |from, to, strength| {
                        let tx = tx.clone();
                        async move {
                            let _ = tx.send((from, to, strength)).await;
                            Ok(())
                        }
                    })
                    .await;
            });

            while let Some(assoc) = rx.recv().await {
                if !first {
                    writer.write_all(b",").await?;
                }
                first = false;
                let data = serde_json::to_vec(&assoc)?;
                writer.write_all(&data).await?;
            }
        } else {
            let sing = self.singularity.read().await;
            if let Some(ns_state) = sing.get_namespace(&ns) {
                for (from_id, neighbors) in &ns_state.associations {
                    for (to_id, strength) in neighbors {
                        if !first {
                            writer.write_all(b",").await?;
                        }
                        first = false;
                        let data = serde_json::to_vec(&(from_id, to_id, *strength))?;
                        writer.write_all(&data).await?;
                    }
                }
            }
        }

        writer.write_all(b"]}}").await?;
        writer.flush().await?;
        file.sync_all().await?;

        Ok(())
    }

    /// Import memory state from JSON file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = tokio::fs::read(validated_path).await?;
        // MAX_IMPORT_SIZE fits in usize on 64-bit
        if bytes.len() > MAX_IMPORT_SIZE as usize {
            return Err(crate::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!(
                    "JSON import data size {} exceeds maximum allowed size {}",
                    bytes.len(),
                    MAX_IMPORT_SIZE
                ),
            });
        }
        let payload: ExportPayload = serde_json::from_slice(&bytes)?;

        if !merge {
            {
                let mut sing = self.singularity.write().await;
                let ns = self.namespace.read().await;
                sing.clear(&ns);
            }
            if let Some(ref persistence) = self.persistence {
                let ns = self.namespace.read().await;
                persistence.clear_namespace(&ns).await?;
            }
        }

        // Acquire write lock, inject concepts + build associations list, then release
        let valid_associations = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            let mut associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(&ns, concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(&ns, from, to, *strength) {
                    Ok(()) => associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import_json"
                        );
                    }
                }
            }
            associations
        }; // Lock released here
        // Persist concepts and associations (no lock needed)
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            persistence.save_concepts(&ns, &payload.concepts).await?;
            persistence
                .save_associations(&ns, &valid_associations)
                .await?;
        }
        Ok(payload.concepts.len())
    }

    /// Export memory state to binary file using streaming.
    #[allow(clippy::significant_drop_tightening)]
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_binary(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let mut file = File::create(validated_path).await?;
        let mut writer = BufWriter::new(&mut file);

        let version = env!("CARGO_PKG_VERSION").to_string();
        let exported_at = unix_now_secs();

        let ns = self.namespace.read().await.clone();
        let (concept_count, assoc_count) = if let Some(ref persistence) = self.persistence {
            let c_count = {
                let sing = self.singularity.read().await;
                sing.len(&ns)
            };
            let a_count = persistence.association_count(&ns).await?;
            (c_count, a_count)
        } else {
            let sing = self.singularity.read().await;
            (
                sing.len(&ns),
                sing.get_namespace(&ns)
                    .map(|n| n.associations.values().map(|m| m.len()).sum())
                    .unwrap_or(0),
            )
        };

        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);

        let mut header_buf = Vec::new();
        options
            .serialize_into(&mut header_buf, &version)
            .map_err(|e| crate::error::MemoryError::database(e.to_string()))?;
        options
            .serialize_into(&mut header_buf, &exported_at)
            .map_err(|e| crate::error::MemoryError::database(e.to_string()))?;
        options
            .serialize_into(&mut header_buf, &(concept_count as u64))
            .map_err(|e| crate::error::MemoryError::database(e.to_string()))?;
        writer.write_all(&header_buf).await?;

        if let Some(ref persistence) = self.persistence {
            let (tx, mut rx) = mpsc::channel(32);
            let persistence_clone = persistence.clone();
            let ns_clone = ns.clone();

            tokio::spawn(async move {
                let _ = persistence_clone
                    .for_each_concept_scoped(&ns_clone, |concept| {
                        let tx = tx.clone();
                        async move {
                            let _ = tx.send(concept).await;
                            Ok(())
                        }
                    })
                    .await;
            });

            while let Some(concept) = rx.recv().await {
                let binary_concept = BinaryConcept::from(concept);
                let data = options.serialize(&binary_concept).map_err(|e| {
                    crate::error::MemoryError::Persistence(format!("Serialization error: {}", e))
                })?;
                writer.write_all(&data).await?;
            }
        } else {
            let sing = self.singularity.read().await;
            if let Some(ns_state) = sing.get_namespace(&ns) {
                for concept in ns_state.concepts.values() {
                    let binary_concept = BinaryConcept::from(concept.clone());
                    let data = options.serialize(&binary_concept).map_err(|e| {
                        crate::error::MemoryError::Persistence(format!("Serialization error: {}", e))
                    })?;
                    writer.write_all(&data).await?;
                }
            }
        }

        let mut assoc_header = Vec::new();
        options
            .serialize_into(&mut assoc_header, &(assoc_count as u64))
            .map_err(|e| crate::error::MemoryError::database(e.to_string()))?;
        writer.write_all(&assoc_header).await?;

        if let Some(ref persistence) = self.persistence {
            let (tx, mut rx) = mpsc::channel(32);
            let persistence_clone = persistence.clone();
            let ns_clone = ns.clone();

            tokio::spawn(async move {
                let _ = persistence_clone
                    .for_each_association_scoped(&ns_clone, |from, to, strength| {
                        let tx = tx.clone();
                        async move {
                            let _ = tx.send((from, to, strength)).await;
                            Ok(())
                        }
                    })
                    .await;
            });

            while let Some(assoc) = rx.recv().await {
                let data = options.serialize(&assoc).map_err(|e| {
                    crate::error::MemoryError::Persistence(format!("Serialization error: {}", e))
                })?;
                writer.write_all(&data).await?;
            }
        } else {
            let sing = self.singularity.read().await;
            if let Some(ns_state) = sing.get_namespace(&ns) {
                for (from_id, neighbors) in &ns_state.associations {
                    for (to_id, strength) in neighbors {
                        let data = options.serialize(&(from_id, to_id, *strength)).map_err(|e| {
                            crate::error::MemoryError::Persistence(format!(
                                "Serialization error: {}",
                                e
                            ))
                        })?;
                        writer.write_all(&data).await?;
                    }
                }
            }
        }

        writer.flush().await?;
        file.sync_all().await?;
        Ok(())
    }

    /// Import memory state from binary file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = tokio::fs::read(validated_path).await?;

        // MAX_IMPORT_SIZE fits in usize on 64-bit
        if bytes.len() > MAX_IMPORT_SIZE as usize {
            return Err(crate::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!(
                    "import data size {} exceeds maximum allowed size {}",
                    bytes.len(),
                    MAX_IMPORT_SIZE
                ),
            });
        }
        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let binary_payload: BinaryExportPayload =
            options
                .deserialize(&bytes)
                .map_err(|e| crate::error::MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("bincode deserialization failed: {e}"),
                })?;
        // Convert to regular payload
        let payload = binary_payload.to_export_payload().map_err(|e| {
            crate::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("failed to convert binary payload: {e}"),
            }
        })?;
        if !merge {
            {
                let mut sing = self.singularity.write().await;
                let ns = self.namespace.read().await;
                sing.clear(&ns);
            }
            if let Some(ref persistence) = self.persistence {
                let ns = self.namespace.read().await;
                persistence.clear_namespace(&ns).await?;
            }
        }
        // Acquire write lock, inject concepts + build associations list, then release
        let valid_associations = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            let mut associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(&ns, concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(&ns, from, to, *strength) {
                    Ok(()) => associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import_binary"
                        );
                    }
                }
            }
            associations
        }; // Lock released here

        // Persist concepts and associations (no lock needed)
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            persistence.save_concepts(&ns, &payload.concepts).await?;
            persistence
                .save_associations(&ns, &valid_associations)
                .await?;
        }

        Ok(payload.concepts.len())
    }
}
