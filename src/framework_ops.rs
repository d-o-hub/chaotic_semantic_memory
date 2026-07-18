#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
use crate::export_payload::{BinaryExportPayload, ExportPayload, unix_now_secs};
use crate::framework::ChaoticSemanticFramework;
use crate::framework_events::MemoryEvent;
use crate::framework_validation::{MAX_IMPORT_SIZE, validate_path};
use crate::singularity::ConceptBuilder;
use bincode::Options;
use csm_core::error::Result;
use csm_core::hyperdim::HVec10240;
use std::sync::Arc;
use tokio::fs;
use tracing::{instrument, warn};

// Singularity is Send + Sync for Rayon probe/inject construction (#525 / probe_batch).
const _: () = {
    const fn assert_sync_send<T: Sync + Send>() {}
    assert_sync_send::<crate::singularity::Singularity>();
};

const MAX_HISTORY_LIMIT: usize = 1000;

impl ChaoticSemanticFramework {
    /// Batch inject: build concepts (optionally parallel) then durable commit.
    /// Construction runs **before** any write lock (`durable_inject_concepts`).
    #[instrument(err, skip(self, concepts))]
    pub async fn inject_concepts(&self, concepts: &[(String, HVec10240)]) -> Result<()> {
        self.validate_batch_size(concepts.len())?;
        if concepts.is_empty() {
            return Ok(());
        }

        // #525: parallel CPU-bound ConceptBuilder work outside the write lock.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let to_save: Vec<_> = {
            use rayon::prelude::*;
            concepts
                .par_iter()
                .map(|(id, vector)| {
                    Self::validate_concept_id(id)?;
                    ConceptBuilder::new(id.clone()).with_vector(*vector).build()
                })
                .collect::<Result<Vec<_>>>()?
        };
        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let to_save: Vec<_> = {
            let mut out = Vec::with_capacity(concepts.len());
            for (id, vector) in concepts {
                Self::validate_concept_id(id)?;
                out.push(
                    ConceptBuilder::new(id.clone())
                        .with_vector(*vector)
                        .build()?,
                );
            }
            out
        };

        #[cfg(not(target_arch = "wasm32"))]
        let p_start = std::time::Instant::now();
        self.durable_inject_concepts(&to_save).await?;
        if self.persistence.is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = 0u64;
            self.metrics.observe_persist_latency_ms(elapsed_ms, "save");
        }
        self.metrics.inc_concepts_injected(to_save.len() as u64);
        Ok(())
    }

    /// Batch create associations between concepts.
    #[instrument(err, skip(self, associations))]
    pub async fn associate_many(&self, associations: &[(String, String, f32)]) -> Result<()> {
        self.validate_batch_size(associations.len())?;
        if associations.is_empty() {
            return Ok(());
        }
        // #524: clone namespace once (tokio guard cannot cross await).
        let ns = self.namespace().await;
        {
            let mut sing = self.singularity.write().await;
            for (from, to, strength) in associations {
                Self::validate_concept_id(from)?;
                Self::validate_concept_id(to)?;
                Self::validate_association_strength(*strength)?;
                sing.associate(&ns, from, to, *strength)?;
            }
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_associations(&ns, associations).await?;
        }

        self.metrics
            .inc_associations_created(associations.len() as u64);
        Ok(())
    }

    /// Batch similarity queries without caching.
    #[instrument(err, skip(self, queries))]
    pub async fn probe_batch(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Vec<(String, f32)>>> {
        self.validate_top_k(top_k)?;
        self.validate_batch_size(queries.len())?;
        let out = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;

            #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
            {
                use rayon::prelude::*;
                queries
                    .par_iter()
                    .map(|q| sing.find_similar(&ns, q, top_k))
                    .collect()
            }

            #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
            {
                queries
                    .iter()
                    .map(|q| sing.find_similar(&ns, q, top_k))
                    .collect()
            }
        };
        Ok(out)
    }

    /// Batch similarity queries with LRU caching.
    #[allow(clippy::type_complexity)]
    #[instrument(err, skip(self, queries))]
    pub async fn probe_batch_cached(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Arc<[(String, f32)]>>> {
        self.validate_top_k(top_k)?;
        self.validate_batch_size(queries.len())?;
        let out = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;

            #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
            {
                use rayon::prelude::*;
                queries
                    .par_iter()
                    .map(|q| sing.find_similar_cached(&ns, q, top_k))
                    .collect()
            }

            #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
            {
                queries
                    .iter()
                    .map(|q| sing.find_similar_cached(&ns, q, top_k))
                    .collect()
            }
        };
        Ok(out)
    }

    /// Export memory state to JSON file.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_json(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;

        let payload = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(&ns),
                associations: sing.all_associations(&ns),
            }
        };
        let data = serde_json::to_vec_pretty(&payload)?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Securely read a file into bytes with size limit (CWE-770).
    pub(crate) async fn secure_read_file(
        &self,
        path: &std::path::Path,
        limit: u64,
    ) -> Result<Vec<u8>> {
        let metadata = fs::metadata(path).await?;
        if metadata.len() > limit {
            return Err(csm_core::error::MemoryError::InvalidInput {
                field: "file_size".to_string(),
                reason: format!(
                    "File size {} exceeds maximum allowed size {}",
                    metadata.len(),
                    limit
                ),
            });
        }
        Ok(fs::read(path).await?)
    }

    /// #526: apply import payload with short write holds.
    ///
    /// Phase 1 (write): inject concepts only.  
    /// Phase 2 (write): associations only so readers can run between phases.
    /// TOCTOU: a concurrent delete between phases may cause associate to fail;
    /// invalid associations are skipped with `warn!` (pre-existing semantics).
    async fn apply_import_payload(
        &self,
        payload: &ExportPayload,
    ) -> Result<Vec<(String, String, f32)>> {
        for concept in &payload.concepts {
            self.validate_concept(concept)?;
        }
        let ns = self.namespace().await;

        {
            let mut sing = self.singularity.write().await;
            for concept in &payload.concepts {
                sing.inject(&ns, concept.clone())?;
            }
        }

        let mut associations = Vec::with_capacity(payload.associations.len());
        {
            let mut sing = self.singularity.write().await;
            for (from, to, strength) in &payload.associations {
                match sing.associate(&ns, from, to, *strength) {
                    Ok(()) => associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import"
                        );
                    }
                }
            }
        }
        Ok(associations)
    }

    async fn clear_for_import_replace(&self) -> Result<()> {
        let ns = self.namespace().await;
        {
            let mut sing = self.singularity.write().await;
            sing.clear(&ns);
        }
        if let Some(ref persistence) = self.persistence {
            persistence.clear_namespace(&ns).await?;
        }
        Ok(())
    }

    async fn persist_import(
        &self,
        payload: &ExportPayload,
        associations: &[(String, String, f32)],
    ) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace().await;
            persistence.save_concepts(&ns, &payload.concepts).await?;
            persistence.save_associations(&ns, associations).await?;
        }
        Ok(())
    }

    /// Import memory state from JSON file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = self
            .secure_read_file(&validated_path, MAX_IMPORT_SIZE)
            .await?;
        let payload: ExportPayload = serde_json::from_slice(&bytes)?;

        if !merge {
            self.clear_for_import_replace().await?;
        }
        let valid_associations = self.apply_import_payload(&payload).await?;
        self.persist_import(&payload, &valid_associations).await?;
        Ok(payload.concepts.len())
    }

    /// Export memory state to binary file.
    #[allow(clippy::significant_drop_tightening)]
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_binary(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;

        let payload = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            let json_payload = ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(&ns),
                associations: sing.all_associations(&ns),
            };
            let res = BinaryExportPayload::from(json_payload);
            drop(sing);
            res
        };

        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let data = options.serialize(&payload).map_err(|e| {
            csm_core::error::MemoryError::Serialization(serde_json::Error::io(
                std::io::Error::other(e.to_string()),
            ))
        })?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Import memory state from binary file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = self
            .secure_read_file(&validated_path, MAX_IMPORT_SIZE)
            .await?;
        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let binary_payload: BinaryExportPayload = options.deserialize(&bytes).map_err(|e| {
            csm_core::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("bincode deserialization failed: {e}"),
            }
        })?;
        let payload = binary_payload.to_export_payload().map_err(|e| {
            csm_core::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("failed to convert binary payload: {e}"),
            }
        })?;
        if !merge {
            self.clear_for_import_replace().await?;
        }
        let valid_associations = self.apply_import_payload(&payload).await?;
        self.persist_import(&payload, &valid_associations).await?;
        Ok(payload.concepts.len())
    }

    /// Create database backup (SQLite only).
    #[instrument(err, skip(self), fields(path))]
    pub async fn backup(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        if let Some(ref persistence) = self.persistence {
            persistence
                .backup(validated_path.to_str().unwrap_or(path))
                .await?;
        }
        Ok(())
    }

    /// Restore from database backup (SQLite only).
    #[instrument(err, skip(self), fields(path))]
    pub async fn restore(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        if let Some(ref persistence) = self.persistence {
            persistence
                .restore(validated_path.to_str().unwrap_or(path))
                .await?;
            self.load_replace().await?;
        }
        Ok(())
    }

    /// Get version history for a concept.
    #[instrument(err, skip(self), fields(id, limit))]
    pub async fn concept_history(
        &self,
        id: &str,
        mut limit: usize,
    ) -> Result<Vec<crate::singularity::ConceptVersion>> {
        if limit > MAX_HISTORY_LIMIT {
            limit = MAX_HISTORY_LIMIT;
        }
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace().await;
            return persistence.get_concept_history(&ns, id, limit).await;
        }
        Ok(Vec::new())
    }

    /// Update a concept's vector.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_vector(&self, id: &str, vector: HVec10240) -> Result<()> {
        Self::validate_concept_id(id)?;
        let ns = self.namespace().await;
        let concept = {
            let mut sing = self.singularity.write().await;
            sing.update(&ns, id, vector)?;
            sing.get(&ns, id).cloned()
        };

        if let (Some(concept), Some(persistence)) = (concept, &self.persistence) {
            persistence.save_concept(&ns, &concept).await?;
        }
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        })
        .await;
        Ok(())
    }

    /// Update a concept's metadata.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_metadata(
        &self,
        id: &str,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Self::validate_concept_id(id)?;
        Self::validate_metadata_bytes(&metadata, self.config.max_metadata_bytes)?;
        let ns = self.namespace().await;
        let concept = {
            let mut sing = self.singularity.write().await;
            sing.update_metadata(&ns, id, metadata)?;
            sing.get(&ns, id).cloned()
        };

        if let (Some(concept), Some(persistence)) = (concept, &self.persistence) {
            persistence.save_concept(&ns, &concept).await?;
        }
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        })
        .await;
        Ok(())
    }

    /// Remove an association between two concepts.
    #[instrument(err, skip(self), fields(from, to))]
    pub async fn disassociate(&self, from: &str, to: &str) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let ns = self.namespace().await;
        {
            let mut sing = self.singularity.write().await;
            sing.disassociate(&ns, from, to)?;
        }

        if let Some(persistence) = &self.persistence {
            persistence.delete_association(&ns, from, to).await?;
        }
        self.emit_event(MemoryEvent::Disassociated {
            from: from.to_string(),
            to: to.to_string(),
        })
        .await;
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    #[instrument(err, skip(self), fields(id))]
    pub async fn clear_associations(&self, id: &str) -> Result<()> {
        Self::validate_concept_id(id)?;
        let ns = self.namespace().await;
        {
            let mut sing = self.singularity.write().await;
            sing.clear_associations(&ns, id)?;
        }

        if let Some(persistence) = &self.persistence {
            persistence.clear_concept_associations(&ns, id).await?;
        }
        Ok(())
    }

    /// Clear the similarity query cache.
    pub async fn clear_similarity_cache(&self) {
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        sing.invalidate_cache(&ns);
    }

    /// Bundle multiple concepts into a single hypervector (strict version).
    pub async fn bundle_concepts_strict(&self, ids: &[String]) -> Result<HVec10240> {
        self.validate_batch_size(ids.len())?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        sing.bundle_concepts_strict(&ns, ids)
    }
}
