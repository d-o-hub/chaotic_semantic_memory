//! Framework persistence operations.
//!
//! Extracted from framework.rs to satisfy the 500 LOC gate.
//! ADR-0093: rows are authoritative; ANN snapshots are revisioned derivatives;
//! state locks are not held across persistence I/O.

use tracing::warn;

use crate::framework::ChaoticSemanticFramework;
use crate::framework_events::MemoryEvent;
use crate::index_envelope::{IndexSnapshotEnvelope, backend_fingerprint};
use crate::singularity::{Concept, ConceptDiff, ConceptVersion};
use csm_core::error::{MemoryError, Result};

impl ChaoticSemanticFramework {
    /// Persist all data to storage (checkpoint + revisioned ANN snapshot).
    #[tracing::instrument(err, skip(self))]
    pub async fn persist(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            #[cfg(not(target_arch = "wasm32"))]
            let p_start = std::time::Instant::now();

            // Snapshot under short locks; re-check revision after unlock so we never
            // stamp a current revision onto an incomplete index (TOCTOU, ADR-0093).
            let ns = self.namespace.read().await.clone();
            let rev_before = persistence.get_namespace_revision(&ns).await?;
            let (index_data, fingerprint) = {
                let sing = self.singularity.read().await;
                let fingerprint = backend_fingerprint(&sing.config.index_backend);
                let index_data = match sing.get_namespace(&ns) {
                    Some(ns_state) => match ns_state.index.serialize() {
                        Ok(data) if !data.is_empty() => Some(data),
                        Ok(_) => None,
                        Err(e) => {
                            warn!(error = %e, "ANN index serialize failed; skipping envelope");
                            None
                        }
                    },
                    None => None,
                };
                drop(sing);
                (index_data, fingerprint)
            };

            if let Some(index_data) = index_data {
                let rev_after = persistence.get_namespace_revision(&ns).await?;
                if rev_after == rev_before {
                    let envelope = IndexSnapshotEnvelope::new(rev_after, fingerprint, index_data);
                    persistence
                        .save_index_envelope(&ns, "main", &envelope)
                        .await?;
                } else {
                    warn!(
                        rev_before,
                        rev_after,
                        "namespace revision moved during ANN snapshot; skipping envelope write"
                    );
                }
            }

            persistence.checkpoint().await?;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.metrics.observe_persist_latency_ms(
                    u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "persist",
                );
            }
        }
        Ok(())
    }

    /// Verify persistence connectivity.
    #[tracing::instrument(err, skip(self))]
    pub async fn persistence_health_check(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            persistence.health_check().await?;
        }
        Ok(())
    }

    /// Load and replace all in-memory state from persistence.
    ///
    /// Clears existing state, loads persisted state. Use for fresh starts.
    /// See also: [`load_merge`](Self::load_merge) for additive semantics.
    // Lock held for inject + rebuild after all I/O is complete (ADR-0093).
    #[allow(clippy::significant_drop_tightening)]
    #[tracing::instrument(err, skip(self))]
    pub async fn load_replace(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            #[cfg(not(target_arch = "wasm32"))]
            let p_start = std::time::Instant::now();

            // Namespace string + all durable I/O without holding singularity locks.
            let ns = self.namespace.read().await.clone();
            let concepts = persistence.load_all_concepts(&ns).await?;
            for concept in &concepts {
                self.validate_concept(concept)?;
            }
            let all_associations = persistence.load_all_associations(&ns).await?;
            let revision = persistence.get_namespace_revision(&ns).await?;
            // Corrupt/legacy envelopes degrade to rebuild (rows authoritative).
            let envelope = match persistence.load_index_envelope(&ns, "main").await {
                Ok(env) => env,
                Err(e) => {
                    warn!(
                        error = %e,
                        "ANN envelope unreadable; rebuilding index from concept rows"
                    );
                    None
                }
            };

            let expected_fingerprint = {
                let sing = self.singularity.read().await;
                backend_fingerprint(&sing.config.index_backend)
            };

            {
                let mut sing = self.singularity.write().await;
                sing.clear(&ns);
                for concept in concepts {
                    sing.inject(&ns, concept)?;
                }
                for (from_id, to_id, strength, created_at) in all_associations {
                    let ns_state = sing.ensure_namespace(&ns)?;
                    let neighbors = ns_state.associations.entry(from_id).or_default();
                    neighbors.insert(to_id, (strength, created_at));
                }

                let apply_snapshot = envelope.as_ref().is_some_and(|env| {
                    env.namespace_revision == revision
                        && env.backend_fingerprint == expected_fingerprint
                });

                if apply_snapshot {
                    if let Some(env) = envelope {
                        let ns_state = sing.ensure_namespace(&ns)?;
                        if let Err(e) = ns_state.index.deserialize(&env.index_data) {
                            warn!(error = %e, "ANN snapshot deserialize failed; rebuilding");
                            let concepts_map = ns_state.concepts.clone();
                            ns_state.index.rebuild(&concepts_map)?;
                        }
                    }
                } else {
                    if envelope.is_some() {
                        warn!(
                            revision,
                            expected = %expected_fingerprint,
                            "rejecting stale or incompatible ANN snapshot; rebuilding from rows"
                        );
                    }
                    let ns_state = sing.ensure_namespace(&ns)?;
                    let concepts_map = ns_state.concepts.clone();
                    ns_state.index.rebuild(&concepts_map)?;
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                self.metrics.observe_persist_latency_ms(
                    u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "load",
                );
            }
        }
        Ok(())
    }

    /// Load and merge persisted state into in-memory state.
    ///
    /// Preserves existing state, adds persisted state on top.
    /// Never applies a persisted-only ANN snapshot over the merged union (ADR-0093).
    // Lock held for inject + rebuild after I/O (ADR-0093).
    #[allow(clippy::significant_drop_tightening)]
    #[tracing::instrument(err, skip(self))]
    pub async fn load_merge(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await.clone();
            let concepts = persistence.load_all_concepts(&ns).await?;
            for concept in &concepts {
                self.validate_concept(concept)?;
            }
            let all_associations = persistence.load_all_associations(&ns).await?;

            {
                let mut sing = self.singularity.write().await;
                for concept in &concepts {
                    if sing.get(&ns, &concept.id).is_some() {
                        warn!(
                            concept_id = %concept.id,
                            "skipping persisted concept during load_merge because id already exists in memory"
                        );
                        continue;
                    }
                    sing.inject(&ns, (*concept).clone())?;
                }
                for (from_id, to_id, strength, created_at) in all_associations {
                    let ns_state = sing.ensure_namespace(&ns)?;
                    let neighbors = ns_state.associations.entry(from_id).or_default();
                    neighbors.insert(to_id, (strength, created_at));
                }
                // Rebuild from final union; never replace with a persisted-only snapshot.
                let ns_state = sing.ensure_namespace(&ns)?;
                let concepts_map = ns_state.concepts.clone();
                ns_state.index.rebuild(&concepts_map)?;
            }
        }
        Ok(())
    }

    /// Batch durable inject (ADR-0093): commit all rows, then apply memory.
    #[allow(clippy::significant_drop_tightening)] // sequential inject under one write lock
    pub(crate) async fn durable_inject_concepts(&self, concepts: &[Concept]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace().await;
            persistence.save_concepts(&ns, concepts).await?;
            let mut sing = self.singularity.write().await;
            for concept in concepts {
                if let Err(e) = sing.inject(&ns, concept.clone()) {
                    drop(sing);
                    warn!(
                        error = %e,
                        "post-commit batch inject failed; reloading namespace"
                    );
                    self.reload_namespace_from_rows(&ns).await?;
                    return Ok(());
                }
            }
        } else {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            for concept in concepts {
                sing.inject(&ns, concept.clone())?;
            }
        }
        Ok(())
    }

    /// Commit a concept to durable storage first, then apply to memory (ADR-0093).
    ///
    /// Persistence failures leave in-memory state unchanged. A rare post-commit
    /// memory apply failure reloads the namespace from authoritative rows.
    pub(crate) async fn durable_inject_concept(&self, concept: Concept) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace().await;
            persistence.save_concept(&ns, &concept).await?;
            let mut sing = self.singularity.write().await;
            if let Err(e) = sing.inject(&ns, concept.clone()) {
                drop(sing);
                warn!(
                    error = %e,
                    concept_id = %concept.id,
                    "post-commit memory inject failed; reloading namespace from durable rows"
                );
                // Durable commit succeeded; reconcile memory and return Ok so
                // callers do not treat a successful write as failure.
                self.reload_namespace_from_rows(&ns).await?;
            }
        } else {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.inject(&ns, concept)?;
        }
        Ok(())
    }

    /// Durable delete: commit row removal first, then memory (ADR-0093).
    pub(crate) async fn durable_delete_concept(&self, id: &str) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace().await;
            persistence.delete_concept(&ns, id).await?;
            let mut sing = self.singularity.write().await;
            if let Err(e) = sing.delete(&ns, id) {
                drop(sing);
                warn!(
                    error = %e,
                    concept_id = %id,
                    "post-commit memory delete failed; reloading namespace from durable rows"
                );
                self.reload_namespace_from_rows(&ns).await?;
                return Err(MemoryError::database(format!(
                    "in-memory delete failed after durable commit for concept {id}; namespace reloaded: {e}"
                )));
            }
        } else {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.delete(&ns, id)?;
        }
        Ok(())
    }

    /// Reload one namespace from authoritative concept/association rows.
    #[allow(clippy::significant_drop_tightening)] // inject + rebuild under one write lock
    pub(crate) async fn reload_namespace_from_rows(&self, ns: &str) -> Result<()> {
        let Some(ref persistence) = self.persistence else {
            return Ok(());
        };
        let concepts = persistence.load_all_concepts(ns).await?;
        let associations = persistence.load_all_associations(ns).await?;
        let mut sing = self.singularity.write().await;
        sing.clear(ns);
        for concept in concepts {
            sing.inject(ns, concept)?;
        }
        for (from_id, to_id, strength, created_at) in associations {
            let ns_state = sing.ensure_namespace(ns)?;
            let neighbors = ns_state.associations.entry(from_id).or_default();
            neighbors.insert(to_id, (strength, created_at));
        }
        let ns_state = sing.ensure_namespace(ns)?;
        let concepts_map = ns_state.concepts.clone();
        ns_state.index.rebuild(&concepts_map)?;
        Ok(())
    }

    /// List all historical versions of a concept.
    #[tracing::instrument(err, skip(self))]
    pub async fn list_versions(&self, id: &str) -> Result<Vec<ConceptVersion>> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            persistence.list_versions_scoped(&ns, id).await
        } else {
            Err(MemoryError::UnsupportedOperation(
                "Persistence is required for version history".to_string(),
            ))
        }
    }

    /// Load a specific concept version.
    #[tracing::instrument(err, skip(self))]
    pub async fn get_version(&self, id: &str, version: u64) -> Result<Option<Concept>> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            persistence.get_version_scoped(&ns, id, version).await
        } else {
            Err(MemoryError::UnsupportedOperation(
                "Persistence is required to retrieve a concept version".to_string(),
            ))
        }
    }

    /// Calculate differences between two versions of a concept.
    #[tracing::instrument(err, skip(self))]
    pub async fn diff_versions(
        &self,
        id: &str,
        from_version: u64,
        to_version: u64,
    ) -> Result<ConceptDiff> {
        let from_concept =
            self.get_version(id, from_version)
                .await?
                .ok_or_else(|| MemoryError::NotFound {
                    entity: "ConceptVersion".to_string(),
                    id: format!("{id}@{from_version}"),
                })?;
        let to_concept =
            self.get_version(id, to_version)
                .await?
                .ok_or_else(|| MemoryError::NotFound {
                    entity: "ConceptVersion".to_string(),
                    id: format!("{id}@{to_version}"),
                })?;
        Ok(ConceptDiff::calculate(&from_concept, &to_concept))
    }

    /// Roll back a concept to a historical version.
    /// Rollbacks must not delete history; they must inject the old concept state as a new head version.
    #[tracing::instrument(err, skip(self))]
    pub async fn rollback_to_version(&self, id: &str, version: u64) -> Result<Concept> {
        let mut target_concept =
            self.get_version(id, version)
                .await?
                .ok_or_else(|| MemoryError::NotFound {
                    entity: "ConceptVersion".to_string(),
                    id: format!("{id}@{version}"),
                })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| MemoryError::database(format!("System clock error: {e}")))?
            .as_secs();
        target_concept.modified_at = now;

        #[cfg(not(target_arch = "wasm32"))]
        let p_start = std::time::Instant::now();
        self.durable_inject_concept(target_concept.clone()).await?;
        #[cfg(not(target_arch = "wasm32"))]
        if self.persistence.is_some() {
            self.metrics.observe_persist_latency_ms(
                u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                "save",
            );
        }

        self.emit_event(MemoryEvent::ConceptInjected {
            id: target_concept.id.clone(),
            timestamp: target_concept.modified_at,
        })
        .await;

        Ok(target_concept)
    }
}
