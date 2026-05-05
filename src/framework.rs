//! Main framework integrating all components

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

use crate::error::Result;
use crate::framework_builder::{FrameworkBuilder, FrameworkConfig, FrameworkStats};
use crate::framework_events::MemoryEvent;
use crate::framework_metrics::{FrameworkMetrics, FrameworkMetricsSnapshot};
use crate::graph_traversal::TraversalConfig;
use crate::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
#[cfg(feature = "persistence")]
use crate::persistence::Persistence;
use crate::reservoir::ChaoticReservoir;
use crate::singularity::{Concept, ConceptBuilder, Singularity, unix_now_secs};

/// Main framework for chaotic semantic memory
pub struct ChaoticSemanticFramework {
    pub(crate) singularity: Arc<RwLock<Singularity>>,
    #[cfg(feature = "persistence")]
    pub(crate) persistence: Option<Arc<Persistence>>,
    #[cfg(not(feature = "persistence"))]
    pub(crate) persistence: Option<Arc<crate::persistence::Persistence>>,
    pub(crate) reservoir: Arc<RwLock<Option<ChaoticReservoir>>>,
    pub(crate) config: FrameworkConfig,
    pub(crate) metrics: Arc<FrameworkMetrics>,
    pub(crate) event_sender: tokio::sync::broadcast::Sender<MemoryEvent>,
    pub(crate) namespace: String,
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
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Create a new framework instance pointing to a different namespace.
    /// This shares the same underlying singularity, persistence and reservoir.
    pub fn with_namespace(&self, ns: impl Into<String>) -> Self {
        Self {
            singularity: self.singularity.clone(),
            persistence: self.persistence.clone(),
            reservoir: self.reservoir.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            event_sender: self.event_sender.clone(),
            namespace: ns.into(),
        }
    }

    /// Inject a concept into memory
    #[instrument(err, skip(self, id, vector))]
    pub async fn inject_concept(&self, id: impl Into<String>, vector: HVec10240) -> Result<()> {
        let id = id.into();
        Self::validate_concept_id(&id)?;
        let concept = ConceptBuilder::new(id.clone())
            .with_vector(vector)
            .build()?;

        {
            let mut sing = self.singularity.write().await;
            sing.inject(&self.namespace, concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concept(&self.namespace, &concept).await?;
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id,
            timestamp: concept.modified_at,
        });

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

        let mut builder = ConceptBuilder::new(id).with_vector(vector);
        for (key, value) in metadata {
            builder = builder.with_metadata(key, value);
        }
        let concept = builder.build()?;

        {
            let mut sing = self.singularity.write().await;
            sing.inject(&self.namespace, concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concept(&self.namespace, &concept).await?;
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id: concept.id.clone(),
            timestamp: concept.modified_at,
        });

        Ok(())
    }

    /// Query for similar concepts
    #[allow(clippy::significant_drop_tightening)] // Lock needed for expired concept filtering
    #[instrument(err, skip(self, query))]
    pub async fn probe(&self, query: HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.validate_top_k(top_k)?;
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();

        // Acquire lock, get results, release immediately
        let (results, expired_ids) = {
            let sing = self.singularity.read().await;
            let results = sing.find_similar(&self.namespace, &query, top_k);

            // Collect expired IDs while holding lock
            let now = crate::singularity::unix_now_secs();
            let expired_ids: std::collections::HashSet<String> = results
                .iter()
                .filter_map(|(id, _)| {
                    sing.get(&self.namespace, id)
                        .and_then(|c| c.expires_at.filter(|exp| *exp <= now))
                        .map(|_| id.clone())
                })
                .collect();
            (results, expired_ids)
        };

        #[cfg(not(target_arch = "wasm32"))]
        #[allow(clippy::cast_possible_truncation)] // Duration millis to u64 for metrics
        let elapsed_ms = start.elapsed().as_millis() as u64;
        #[cfg(target_arch = "wasm32")]
        let elapsed_ms = 0;
        self.metrics.observe_probe_latency_ms(elapsed_ms);

        // Filter expired concepts without lock
        let filtered: Vec<(String, f32)> = results
            .into_iter()
            .filter(|(id, _)| !expired_ids.contains(id))
            .collect();

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

        // Acquire lock, get results, release immediately
        let results = {
            let sing = self.singularity.read().await;
            sing.find_similar_filtered(&self.namespace, query, top_k, filter)
        };

        #[cfg(not(target_arch = "wasm32"))]
        #[allow(clippy::cast_possible_truncation)] // Duration millis to u64 for metrics
        let elapsed_ms = start.elapsed().as_millis() as u64;
        #[cfg(target_arch = "wasm32")]
        let elapsed_ms = 0;
        self.metrics.observe_probe_latency_ms(elapsed_ms);
        Ok(results.as_ref().to_vec())
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
        sing.bfs(&self.namespace, start, &config)
    }

    /// Find shortest weighted path between two concepts.
    #[instrument(err, skip(self))]
    pub async fn shortest_path(&self, from: &str, to: &str) -> Result<Option<Vec<String>>> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let sing = self.singularity.read().await;
        sing.shortest_path(&self.namespace, from, to, &TraversalConfig::default())
    }

    /// Process temporal sequence through reservoir
    #[allow(clippy::significant_drop_tightening)] // Reservoir lock needed for sequence processing
    #[instrument(err, skip(self, sequence))]
    pub async fn process_sequence(&self, sequence: &[Vec<f32>]) -> Result<HVec10240> {
        self.validate_sequence_length(sequence.len())?;
        let mut reservoir = self.reservoir.write().await;

        if reservoir.is_none() {
            *reservoir = Some(ChaoticReservoir::new(
                self.config.reservoir_input_size,
                self.config.reservoir_size,
                self.config.chaos_strength,
            )?);
        }

        let r = reservoir
            .as_mut()
            .ok_or(crate::error::MemoryError::reservoir(
                "reservoir failed to initialize".to_string(),
            ))?;
        r.reset();
        for input in sequence {
            r.step(input)?;
        }

        r.to_hypervector()
    }

    /// Associate two concepts
    #[instrument(err, skip(self))]
    pub async fn associate(&self, from: &str, to: &str, strength: f32) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        Self::validate_association_strength(strength)?;
        {
            let mut sing = self.singularity.write().await;
            sing.associate(&self.namespace, from, to, strength)?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence
                .save_association(&self.namespace, from, to, strength)
                .await?;
        }
        self.metrics.inc_associations_created(1);
        self.emit_event(MemoryEvent::Associated {
            from: from.to_string(),
            to: to.to_string(),
            strength,
        });

        Ok(())
    }

    /// Delete concept from memory and persistence
    #[instrument(err, skip(self))]
    pub async fn delete_concept(&self, id: &str) -> Result<()> {
        Self::validate_concept_id(id)?;
        {
            let mut sing = self.singularity.write().await;
            sing.delete(&self.namespace, id)?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.delete_concept(&self.namespace, id).await?;
        }

        self.emit_event(MemoryEvent::ConceptDeleted {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        });

        Ok(())
    }

    /// Get associations for a concept (outbound edges).
    #[instrument(err, skip(self))]
    pub async fn get_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        Self::validate_concept_id(id)?;
        let sing = self.singularity.read().await;
        Ok(sing.get_associations(&self.namespace, id))
    }

    /// Get incoming associations for a concept (inbound edges).
    ///
    /// Returns concepts that have associations pointing to this concept,
    /// sorted by strength descending.
    #[instrument(err, skip(self))]
    pub async fn incoming_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        Self::validate_concept_id(id)?;
        let sing = self.singularity.read().await;
        Ok(sing
            .incoming_associations(&self.namespace, id))
    }

    /// Find the fewest-hop path between two concepts (unweighted BFS).
    ///
    /// Returns the path with the minimum number of hops, ignoring edge strengths.
    /// Use [`Self::shortest_path`] for strength-weighted (Dijkstra) traversal.
    #[instrument(err, skip(self))]
    pub async fn shortest_path_hops(&self, from: &str, to: &str) -> Result<Option<Vec<String>>> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let sing = self.singularity.read().await;
        sing.shortest_path_hops(&self.namespace, from, to, &TraversalConfig::default())
    }

    /// Get a concept by ID.
    #[instrument(err, skip(self))]
    pub async fn get_concept(&self, id: &str) -> Result<Option<Concept>> {
        Self::validate_concept_id(id)?;
        let sing = self.singularity.read().await;
        Ok(sing.get(&self.namespace, id).cloned())
    }

    /// Backward-compatible alias for replace semantics.
    ///
    /// Delegates to [`load_replace`](Self::load_replace).
    pub async fn load(&self) -> Result<()> {
        self.load_replace().await
    }

    pub async fn metrics_snapshot(&self) -> FrameworkMetricsSnapshot {
        let mut snapshot = self.metrics.snapshot();

        let cache_snapshot = {
            let sing = self.singularity.read().await;
            sing.cache_metrics_snapshot(&self.namespace)
        };

        let reservoir_snapshot = {
            let reservoir = self.reservoir.read().await;
            reservoir
                .as_ref()
                .map(ChaoticReservoir::metrics_snapshot)
                .unwrap_or_default()
        };

        snapshot.cache_hits_total = cache_snapshot.cache_hits_total;
        snapshot.cache_misses_total = cache_snapshot.cache_misses_total;
        snapshot.cache_evictions_total = cache_snapshot.cache_evictions_total;
        snapshot.reservoir_steps_total = reservoir_snapshot.reservoir_steps_total;
        snapshot.avg_reservoir_step_latency_us = reservoir_snapshot.avg_reservoir_step_latency_us;
        snapshot.reservoir_nodes_active = reservoir_snapshot.reservoir_nodes_active;
        snapshot
    }

    /// Get framework statistics
    pub async fn stats(&self) -> Result<FrameworkStats> {
        // Get concept count without holding lock during persistence call
        let concept_count = {
            let sing = self.singularity.read().await;
            sing.len(&self.namespace)
        };

        let db_size = if let Some(ref persistence) = self.persistence {
            Some(persistence.size().await.unwrap_or(0))
        } else {
            None
        };

        Ok(FrameworkStats {
            concept_count,
            db_size_bytes: db_size,
        })
    }
}
