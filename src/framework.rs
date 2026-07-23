#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Main framework integrating all components

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::framework_builder::{FrameworkBuilder, FrameworkConfig};
use crate::framework_events::MemoryEvent;
use crate::framework_events_ce::{ChaoticEvent, EventEmitter};
use crate::framework_metrics::FrameworkMetrics;
use crate::graph_traversal::TraversalConfig;
use crate::metadata_filter::MetadataFilter;
#[cfg(feature = "persistence")]
use crate::persistence::Persistence;
use crate::singularity::{ConceptBuilder, Singularity, unix_now_secs};
use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::HVec10240;
use csm_core_lib::reservoir_chaotic::ChaoticReservoir;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;

/// Main framework for chaotic semantic memory
#[derive(Clone)]
pub struct ChaoticSemanticFramework {
    pub(crate) singularity: Arc<RwLock<Singularity>>,
    #[cfg(feature = "persistence")]
    pub(crate) persistence: Option<Arc<Persistence>>,
    #[cfg(not(feature = "persistence"))]
    pub(crate) persistence: Option<Arc<crate::persistence::Persistence>>,
    pub(crate) reservoir: Arc<RwLock<Option<csm_core_lib::reservoir_chaotic::ChaoticReservoir>>>,
    pub(crate) config: FrameworkConfig,
    pub(crate) metrics: Arc<FrameworkMetrics>,
    pub(crate) event_sender: tokio::sync::broadcast::Sender<MemoryEvent>,
    pub(crate) emitters: Vec<Arc<dyn EventEmitter>>,
    pub(crate) namespace: Arc<RwLock<String>>,
    /// Embedding provider for text-to-vector conversion.
    pub(crate) embedding_provider: Arc<dyn crate::embedding::EmbeddingProvider>,
    /// Random projection layer for embedding → HVec mapping.
    pub(crate) projection: Arc<crate::embedding::Projection>,
}

impl ChaoticSemanticFramework {
    /// Create a new framework builder
    #[must_use]
    pub fn builder() -> FrameworkBuilder {
        FrameworkBuilder::new()
    }

    /// Get the singularity (concept store)
    pub fn singularity(&self) -> Arc<RwLock<Singularity>> {
        self.singularity.clone()
    }

    /// Get the current namespace.
    pub async fn namespace(&self) -> String {
        self.namespace.read().await.clone()
    }

    /// Inject a concept into memory
    #[instrument(err, skip(self, id, vector))]
    pub async fn inject_concept(&self, id: impl Into<String>, vector: HVec10240) -> Result<()> {
        let id = id.into();
        Self::validate_concept_id(&id)?;

        let ttl = self.evaluate_ttl_policy(&id, &HashMap::new()).await;

        let mut builder = ConceptBuilder::new(id.clone()).with_vector(vector);
        if let Some(ttl_secs) = ttl {
            builder = builder.with_ttl(ttl_secs);
        }
        let concept = builder.build()?;

        #[cfg(not(target_arch = "wasm32"))]
        let p_start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let p_start = Date::now();

        // ADR-0093: durable commit before in-memory mutation when persistence is enabled.
        self.durable_inject_concept(concept.clone()).await?;

        if self.persistence.is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = (Date::now() - p_start) as u64;
            self.metrics.observe_persist_latency_ms(elapsed_ms, "save");
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id: id.clone(),
            timestamp: concept.modified_at,
        })
        .await;

        self.emit_chaotic_event(ChaoticEvent::BindingCreated {
            key: id,
            dim: HVec10240::DIMENSION,
            target: if self.persistence.is_some() {
                crate::framework_events_ce::StorageTarget::LibSql
            } else {
                crate::framework_events_ce::StorageTarget::Memory
            },
        })
        .await;

        Ok(())
    }

    /// Inject a concept with metadata into memory
    #[instrument(err, skip(self, id, vector, metadata))]
    pub async fn inject_concept_with_metadata(
        &self,
        id: impl Into<String>,
        vector: HVec10240,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let id = id.into();
        Self::validate_concept_id(&id)?;
        Self::validate_metadata_bytes(&metadata, self.config.max_metadata_bytes)?;

        let ttl = self.evaluate_ttl_policy(&id, &metadata).await;

        let mut builder = ConceptBuilder::new(id).with_vector(vector);
        if let Some(ttl_secs) = ttl {
            builder = builder.with_ttl(ttl_secs);
        }
        for (key, value) in metadata {
            builder = builder.with_metadata(key, value);
        }
        let concept = builder.build()?;

        #[cfg(not(target_arch = "wasm32"))]
        let p_start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let p_start = Date::now();

        self.durable_inject_concept(concept.clone()).await?;

        if self.persistence.is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = (Date::now() - p_start) as u64;
            self.metrics.observe_persist_latency_ms(elapsed_ms, "save");
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id: concept.id.clone(),
            timestamp: concept.modified_at,
        })
        .await;

        self.emit_chaotic_event(ChaoticEvent::BindingCreated {
            key: concept.id.clone(),
            dim: HVec10240::DIMENSION,
            target: if self.persistence.is_some() {
                crate::framework_events_ce::StorageTarget::LibSql
            } else {
                crate::framework_events_ce::StorageTarget::Memory
            },
        })
        .await;

        Ok(())
    }

    /// Query for similar concepts
    // Lock needed for expired concept filtering
    #[allow(clippy::significant_drop_tightening)]
    #[instrument(err, skip(self, query))]
    pub async fn probe(&self, query: HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.validate_top_k(top_k)?;
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let start = Date::now();

        // Acquire lock, get results, release immediately
        let (results, expired_ids) = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            let results = sing.find_similar(&ns, &query, top_k);

            let now = crate::singularity::unix_now_secs();
            let expired_ids: std::collections::HashSet<String> = results
                .iter()
                .filter_map(|(id, _)| {
                    sing.get(&ns, id)
                        .and_then(|c| c.expires_at.filter(|exp| *exp <= now))
                        .map(|_| id.clone())
                })
                .collect();
            let res = (results, expired_ids);
            drop(sing);
            res
        };

        #[cfg(not(target_arch = "wasm32"))]
        // Duration millis to u64 for metrics
        let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        #[cfg(target_arch = "wasm32")]
        let elapsed_ms = (Date::now() - start) as u64;
        self.metrics.observe_probe_latency_ms(elapsed_ms);

        // Filter expired concepts without lock
        let filtered: Vec<(String, f32)> = results
            .into_iter()
            .filter(|(id, _)| !expired_ids.contains(id))
            .collect();

        let mut events = Vec::new();
        for (id, similarity) in &filtered {
            if (*similarity as f64) >= self.config.pattern_recognition_threshold {
                events.push(ChaoticEvent::PatternRecognized {
                    query_vector: query.to_bytes(),
                    matched_key: id.clone(),
                    similarity: *similarity as f64,
                });
            }
        }

        for event in events {
            self.emit_chaotic_event(event).await;
        }

        Ok(filtered)
    }

    /// Query for similar concepts with metadata filtering.
    #[instrument(err, skip(self, query, filter))]
    pub async fn probe_filtered(
        &self,
        query: &HVec10240,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> Result<Vec<(String, f32)>> {
        self.validate_top_k(top_k)?;
        Self::validate_metadata_filter(filter)?;
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let start = Date::now();

        // Acquire lock, get results, release immediately
        let results = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            sing.find_similar_filtered(&ns, query, top_k, filter)
        };

        #[cfg(not(target_arch = "wasm32"))]
        // Duration millis to u64 for metrics
        let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        #[cfg(target_arch = "wasm32")]
        let elapsed_ms = (Date::now() - start) as u64;
        self.metrics.observe_probe_latency_ms(elapsed_ms);

        let results_vec = results.as_ref().to_vec();
        let mut events = Vec::new();
        for (id, similarity) in &results_vec {
            if (*similarity as f64) >= self.config.pattern_recognition_threshold {
                events.push(ChaoticEvent::PatternRecognized {
                    query_vector: query.to_bytes(),
                    matched_key: id.clone(),
                    similarity: *similarity as f64,
                });
            }
        }

        for event in events {
            self.emit_chaotic_event(event).await;
        }

        Ok(results_vec)
    }

    /// Traverse graph using breadth-first search.
    #[instrument(err, skip(self, config))]
    pub async fn traverse(
        &self,
        start: &str,
        config: TraversalConfig,
    ) -> Result<Vec<(String, u32)>> {
        Self::validate_concept_id(start)?;
        Self::validate_traversal_config(&config)?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        self.metrics.inc_traversals();
        sing.bfs(&ns, start, &config)
    }

    /// Find shortest weighted path between two concepts.
    #[instrument(err, skip(self))]
    pub async fn shortest_path(&self, from: &str, to: &str) -> Result<Option<Vec<String>>> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        self.metrics.inc_shortest_path();
        sing.shortest_path(&ns, from, to, &TraversalConfig::default())
    }

    /// Process temporal sequence through reservoir
    // Reservoir lock needed for sequence processing
    #[instrument(err, skip(self, sequence))]
    pub async fn process_sequence(&self, sequence: &[Vec<f32>]) -> Result<HVec10240> {
        self.validate_sequence_length(sequence.len())?;

        let mut events = Vec::new();
        let mut reservoir_guard = self.reservoir.write().await;

        if reservoir_guard.is_none() {
            *reservoir_guard = Some(ChaoticReservoir::new_with_metrics(
                self.config.reservoir_input_size,
                self.config.reservoir_size,
                self.config.chaos_strength,
                Arc::clone(&self.metrics.reservoir_metrics),
            )?);
        }

        let r = reservoir_guard
            .as_mut()
            .ok_or(csm_core_lib::error::MemoryError::reservoir(
                "reservoir failed to initialize".to_string(),
            ))?;
        r.reset();

        for (step_idx, input) in sequence.iter().enumerate() {
            let out = r.step(input)?;
            events.push(ChaoticEvent::EchoComputed {
                input_dim: input.len(),
                state_norm: out.state_norm,
            });

            // Convergence detection: if change_norm is very small, we've hit an attractor basin.
            if out.change_norm < 1e-5 {
                events.push(ChaoticEvent::AttractorFired {
                    attractor_id: step_idx as u32,
                    basin_energy: out.change_norm,
                    reservoir_dim: self.config.reservoir_size,
                });
            }
        }
        let hv = r.to_hypervector()?;
        drop(reservoir_guard);

        for event in events {
            self.emit_chaotic_event(event).await;
        }

        Ok(hv)
    }

    /// Associate two concepts
    #[instrument(err, skip(self))]
    pub async fn associate(&self, from: &str, to: &str, strength: f32) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        Self::validate_association_strength(strength)?;
        {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.associate(&ns, from, to, strength)?;
        }

        if let Some(ref persistence) = self.persistence {
            #[cfg(not(target_arch = "wasm32"))]
            let p_start = std::time::Instant::now();
            #[cfg(target_arch = "wasm32")]
            let p_start = Date::now();

            let ns = self.namespace().await;
            persistence
                .save_association(&ns, from, to, strength)
                .await?;

            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = (Date::now() - p_start) as u64;

            self.metrics
                .observe_persist_latency_ms(elapsed_ms, "save_association");
        }
        self.metrics.inc_associations_created(1);
        self.emit_event(MemoryEvent::Associated {
            from: from.to_string(),
            to: to.to_string(),
            strength,
        })
        .await;

        Ok(())
    }

    /// Delete concept from memory and persistence
    #[instrument(err, skip(self))]
    pub async fn delete_concept(&self, id: &str) -> Result<()> {
        Self::validate_concept_id(id)?;
        // ADR-0093: durable delete before in-memory mutation when persistence is enabled.
        self.durable_delete_concept(id).await?;

        self.metrics.inc_delete_concepts(1);
        self.emit_event(MemoryEvent::ConceptDeleted {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        })
        .await;

        Ok(())
    }

    /// Get associations for a concept (outbound edges), with decay applied.
    #[instrument(err, skip(self))]
    pub async fn get_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        Self::validate_concept_id(id)?;
        let curve = self.config.ttl_config.association_decay;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        Ok(sing.get_associations_with_decay(&ns, id, curve))
    }

    /// Get incoming associations for a concept (inbound edges), with decay applied.
    ///
    /// Returns concepts that have associations pointing to this concept,
    /// sorted by strength descending.
    #[instrument(err, skip(self))]
    pub async fn incoming_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        Self::validate_concept_id(id)?;
        let curve = self.config.ttl_config.association_decay;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        Ok(sing
            .incoming_associations_with_decay(&ns, id, curve)
            .into_iter()
            .collect())
    }

    /// Reinforce an association by resetting its decay clock.
    ///
    /// This refreshes the `created_at` timestamp so decay restarts from now,
    /// implementing "use it or lose it" behavior.
    #[instrument(err, skip(self))]
    pub async fn reinforce_association(&self, from: &str, to: &str) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let mut sing = self.singularity.write().await;
        let ns = self.namespace.read().await;
        sing.reinforce_association(&ns, from, to)
    }

    /// Prune associations whose decayed strength falls below `threshold`.
    ///
    /// Uses the configured `association_decay` curve. Returns the count of
    /// pruned associations.
    #[instrument(err, skip(self))]
    pub async fn prune_decayed_associations(&self, threshold: f32) -> Result<usize> {
        let curve = self.config.ttl_config.association_decay;
        let mut sing = self.singularity.write().await;
        let ns = self.namespace.read().await;
        Ok(sing.prune_decayed_associations(&ns, curve, threshold))
    }
}
