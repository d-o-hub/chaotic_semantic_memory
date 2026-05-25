//! Framework export and import operations.
//!
//! Extracted from framework_ops.rs to satisfy the 500 LOC gate.

use bincode::Options;
use tokio::fs;
use tracing::{instrument, warn};
use std::io::BufWriter;
use std::fs::File;

use crate::error::Result;
use crate::export_payload::{BinaryExportPayload, ExportPayload, unix_now_secs};
use crate::framework::ChaoticSemanticFramework;
use crate::framework_validation::validate_path;

pub(crate) const MAX_IMPORT_SIZE: u64 = 100 * 1024 * 1024; // 100 MB default

impl ChaoticSemanticFramework {
    /// Export memory state to JSON file using streaming to avoid OOM.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_json(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let file = File::create(validated_path)?;
        let mut writer = BufWriter::new(file);

        use std::io::Write;

        let version = env!("CARGO_PKG_VERSION");
        let exported_at = unix_now_secs();

        // Write JSON header
        write!(writer, "{{\"version\":\"{}\",\"exported_at\":{},\"concepts\":[", version, exported_at)?;

        let ns = self.namespace.read().await;

        // Stream concepts
        let mut first_concept = true;
        if let Some(ref persistence) = self.persistence {
            persistence.for_each_concept_scoped(&ns, |concept| {
                if !first_concept {
                    let _ = write!(writer, ",");
                }
                first_concept = false;
                let _ = serde_json::to_writer(&mut writer, &concept);
                async { Ok(()) }
            }).await?;
        } else {
            let sing = self.singularity.read().await;
            sing.for_each_concept(&ns, |concept| {
                if !first_concept {
                    let _ = write!(writer, ",");
                }
                first_concept = false;
                let _ = serde_json::to_writer(&mut writer, concept);
            });
        }

        // Write middle separator
        write!(writer, "],\"associations\":[")?;

        // Stream associations
        let mut first_assoc = true;
        if let Some(ref persistence) = self.persistence {
            persistence.for_each_association_scoped(&ns, |from, to, strength| {
                if !first_assoc {
                    let _ = write!(writer, ",");
                }
                first_assoc = false;
                let _ = serde_json::to_writer(&mut writer, &(from, to, strength));
                async { Ok(()) }
            }).await?;
        } else {
            let sing = self.singularity.read().await;
            sing.for_each_association(&ns, |from, to, strength| {
                if !first_assoc {
                    let _ = write!(writer, ",");
                }
                first_assoc = false;
                let _ = serde_json::to_writer(&mut writer, &(from, to, strength));
            });
        }

        // Write JSON footer
        write!(writer, "]}}")?;
        writer.flush()?;

        Ok(())
    }

    /// Import memory state from JSON file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = fs::read(validated_path).await?;
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
        let file = File::create(validated_path)?;
        let mut writer = BufWriter::new(file);

        let version = env!("CARGO_PKG_VERSION").to_string();
        let exported_at = unix_now_secs();

        // Count concepts and associations for bincode Vec headers
        let ns = self.namespace.read().await;
        let (concept_count, assoc_count) = if let Some(ref persistence) = self.persistence {
            // Re-evaluating: Framework::stats() uses sing.len(ns).
            // If we want to export EVERYTHING, we should check persistence.
            let c_count = {
                let sing = self.singularity.read().await;
                sing.len(&ns)
            };
            let a_count = persistence.association_count(&ns).await?;
            (c_count, a_count)
        } else {
            let sing = self.singularity.read().await;
            (sing.len(&ns), sing.association_count(&ns))
        };

        use bincode::Options;
        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);

        // Serialize version and exported_at (matches BinaryExportPayload structure)
        options.serialize_into(&mut writer, &version).map_err(|e| crate::error::MemoryError::database(e.to_string()))?;
        options.serialize_into(&mut writer, &exported_at).map_err(|e| crate::error::MemoryError::database(e.to_string()))?;

        // Serialize concepts Vec header (length as u64 in bincode)
        options.serialize_into(&mut writer, &(concept_count as u64)).map_err(|e| crate::error::MemoryError::database(e.to_string()))?;

        let ns = self.namespace.read().await;

        // Stream concepts
        if let Some(ref persistence) = self.persistence {
            persistence.for_each_concept_scoped(&ns, |concept| {
                let binary_concept = crate::export_payload::BinaryConcept::from(concept);
                let _ = options.serialize_into(&mut writer, &binary_concept);
                async { Ok(()) }
            }).await?;
        } else {
            let sing = self.singularity.read().await;
            sing.for_each_concept(&ns, |concept| {
                let binary_concept = crate::export_payload::BinaryConcept::from(concept.clone());
                let _ = options.serialize_into(&mut writer, &binary_concept);
            });
        }

        // Serialize associations Vec header
        options.serialize_into(&mut writer, &(assoc_count as u64)).map_err(|e| crate::error::MemoryError::database(e.to_string()))?;

        // Stream associations
        if let Some(ref persistence) = self.persistence {
            persistence.for_each_association_scoped(&ns, |from, to, strength| {
                let _ = options.serialize_into(&mut writer, &(from, to, strength));
                async { Ok(()) }
            }).await?;
        } else {
            let sing = self.singularity.read().await;
            sing.for_each_association(&ns, |from, to, strength| {
                let _ = options.serialize_into(&mut writer, &(from, to, strength));
            });
        }

        use std::io::Write;
        writer.flush()?;
        Ok(())
    }

    /// Import memory state from binary file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = fs::read(validated_path).await?;

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
