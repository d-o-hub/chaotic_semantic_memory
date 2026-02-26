//! Main framework integrating all components

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use tracing::{instrument, warn};

use crate::error::Result;
use crate::framework_builder::{FrameworkBuilder, FrameworkConfig, FrameworkStats};
use crate::hyperdim::HVec10240;
use crate::persistence::Persistence;
use crate::reservoir::ChaoticReservoir;
use crate::singularity::{Concept, ConceptBuilder, Singularity};

/// Main framework for chaotic semantic memory
pub struct ChaoticSemanticFramework {
    pub(crate) singularity: Arc<RwLock<Singularity>>,
    pub(crate) persistence: Option<Arc<Persistence>>,
    pub(crate) reservoir: Arc<RwLock<Option<ChaoticReservoir>>>,
    pub(crate) config: FrameworkConfig,
    pub(crate) metrics: Arc<FrameworkMetrics>,
}

#[derive(Debug, Default)]
pub struct FrameworkMetrics {
    concepts_injected_total: AtomicU64,
    associations_created_total: AtomicU64,
    probes_total: AtomicU64,
    probe_latency_ms_total: AtomicU64,
    probe_latency_count: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct FrameworkMetricsSnapshot {
    pub concepts_injected_total: u64,
    pub associations_created_total: u64,
    pub probes_total: u64,
    pub avg_probe_latency_ms: f64,
    pub cache_hits_total: u64,
    pub cache_misses_total: u64,
    pub cache_evictions_total: u64,
    pub reservoir_steps_total: u64,
    pub avg_reservoir_step_latency_us: f64,
    pub reservoir_nodes_active: u64,
}

impl FrameworkMetrics {
    pub(crate) fn inc_concepts_injected(&self, count: u64) {
        self.concepts_injected_total
            .fetch_add(count, Ordering::Relaxed);
    }

    pub(crate) fn inc_associations_created(&self, count: u64) {
        self.associations_created_total
            .fetch_add(count, Ordering::Relaxed);
    }

    fn observe_probe_latency_ms(&self, latency_ms: u64) {
        self.probes_total.fetch_add(1, Ordering::Relaxed);
        self.probe_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.probe_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> FrameworkMetricsSnapshot {
        let count = self.probe_latency_count.load(Ordering::Relaxed);
        let total = self.probe_latency_ms_total.load(Ordering::Relaxed);
        let avg = if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        };

        FrameworkMetricsSnapshot {
            concepts_injected_total: self.concepts_injected_total.load(Ordering::Relaxed),
            associations_created_total: self.associations_created_total.load(Ordering::Relaxed),
            probes_total: self.probes_total.load(Ordering::Relaxed),
            avg_probe_latency_ms: avg,
            cache_hits_total: 0,
            cache_misses_total: 0,
            cache_evictions_total: 0,
            reservoir_steps_total: 0,
            avg_reservoir_step_latency_us: 0.0,
            reservoir_nodes_active: 0,
        }
    }
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

    /// Inject a concept into memory
    #[instrument(skip(self, id, vector))]
    pub async fn inject_concept(&self, id: impl Into<String>, vector: HVec10240) -> Result<()> {
        let id = id.into();
        Self::validate_concept_id(&id)?;
        let concept = ConceptBuilder::new(id).with_vector(vector).build()?;

        {
            let mut sing = self.singularity.write().await;
            sing.inject(concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concept(&concept).await?;
        }
        self.metrics.inc_concepts_injected(1);

        Ok(())
    }

    /// Inject a concept with metadata into memory
    #[instrument(skip(self, id, vector, metadata))]
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
            sing.inject(concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concept(&concept).await?;
        }
        self.metrics.inc_concepts_injected(1);

        Ok(())
    }

    /// Query for similar concepts
    #[instrument(skip(self, query))]
    pub async fn probe(&self, query: HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.validate_top_k(top_k)?;
        let start = std::time::Instant::now();
        let sing = self.singularity.read().await;
        let results = sing.find_similar(&query, top_k);
        let elapsed_ms = start.elapsed().as_millis() as u64;
        self.metrics.observe_probe_latency_ms(elapsed_ms);
        Ok(results)
    }

    /// Process temporal sequence through reservoir
    #[instrument(skip(self, sequence))]
    pub async fn process_sequence(&self, sequence: &[Vec<f32>]) -> Result<HVec10240> {
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
            .ok_or(crate::error::MemoryError::Reservoir(
                "reservoir failed to initialize".to_string(),
            ))?;
        r.reset();
        for input in sequence {
            r.step(input)?;
        }

        r.to_hypervector()
    }

    /// Associate two concepts
    #[instrument(skip(self))]
    pub async fn associate(&self, from: &str, to: &str, strength: f32) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        Self::validate_association_strength(strength)?;
        {
            let mut sing = self.singularity.write().await;
            sing.associate(from, to, strength)?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_association(from, to, strength).await?;
        }
        self.metrics.inc_associations_created(1);

        Ok(())
    }

    /// Delete concept from memory and persistence
    #[instrument(skip(self))]
    pub async fn delete_concept(&self, id: &str) -> Result<()> {
        Self::validate_concept_id(id)?;
        {
            let mut sing = self.singularity.write().await;
            sing.delete(id)?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.delete_concept(id).await?;
        }

        Ok(())
    }

    /// Get associations for a concept
    #[instrument(skip(self))]
    pub async fn get_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        Self::validate_concept_id(id)?;
        let sing = self.singularity.read().await;
        Ok(sing.get_associations(id))
    }

    /// Get a concept by ID.
    #[instrument(skip(self))]
    pub async fn get_concept(&self, id: &str) -> Result<Option<Concept>> {
        Self::validate_concept_id(id)?;
        let sing = self.singularity.read().await;
        Ok(sing.get(id).cloned())
    }

    /// Persist all data to storage
    #[instrument(skip(self))]
    pub async fn persist(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            persistence.checkpoint().await?;
        }
        Ok(())
    }

    /// Verify persistence connectivity.
    #[instrument(skip(self))]
    pub async fn persistence_health_check(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            persistence.health_check().await?;
        }
        Ok(())
    }

    /// Load and replace all in-memory state from persistence
    #[instrument(skip(self))]
    pub async fn load_replace(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let concepts = persistence.load_all_concepts().await?;

            let mut concept_ids = Vec::with_capacity(concepts.len());
            for concept in &concepts {
                self.validate_concept(concept)?;
                concept_ids.push(concept.id.clone());
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept_id in &concept_ids {
                let links = persistence.load_associations(concept_id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept_id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                sing.clear();
                for concept in concepts {
                    sing.inject(concept)?;
                }
                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&from_id, &to_id, strength) {
                        warn!(
                            from_id = %from_id,
                            to_id = %to_id,
                            strength,
                            error = %error,
                            "skipping invalid association during load_replace"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Load and merge persisted state into in-memory state
    #[instrument(skip(self))]
    pub async fn load_merge(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let concepts = persistence.load_all_concepts().await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            let concept_ids = {
                let mut sing = self.singularity.write().await;
                for concept in concepts.clone() {
                    sing.inject(concept)?;
                }
                sing.concept_ids()
            };

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept_id in &concept_ids {
                let links = persistence.load_associations(concept_id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept_id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&from_id, &to_id, strength) {
                        warn!(
                            from_id = %from_id,
                            to_id = %to_id,
                            strength,
                            error = %error,
                            "skipping invalid association during load_merge"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Backward-compatible alias for replace semantics
    pub async fn load(&self) -> Result<()> {
        self.load_replace().await
    }

    pub async fn metrics_snapshot(&self) -> FrameworkMetricsSnapshot {
        let mut snapshot = self.metrics.snapshot();

        let cache_snapshot = {
            let sing = self.singularity.read().await;
            sing.cache_metrics_snapshot()
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
        let sing = self.singularity.read().await;
        let concept_count = sing.len();

        let db_size = if let Some(ref persistence) = self.persistence {
            persistence.size().await.unwrap_or(0)
        } else {
            0
        };

        Ok(FrameworkStats {
            concept_count,
            db_size_bytes: db_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;

    #[tokio::test]
    async fn test_inject_concept_with_metadata_success() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let vector = HVec10240::random();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("source".to_string(), serde_json::json!("test"));
        metadata.insert("score".to_string(), serde_json::json!(0.95));

        framework
            .inject_concept_with_metadata("concept-1", vector, metadata)
            .await
            .unwrap();

        let concept = framework.get_concept("concept-1").await.unwrap().unwrap();
        assert_eq!(concept.id, "concept-1");
        assert_eq!(concept.metadata.get("source").unwrap(), "test");
        assert_eq!(concept.metadata.get("score").unwrap(), 0.95);
    }

    #[tokio::test]
    async fn test_inject_concept_with_metadata_exceeds_limit() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .with_max_metadata_bytes(10)
            .build()
            .await
            .unwrap();

        let vector = HVec10240::random();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "data".to_string(),
            serde_json::json!("this is a very long string"),
        );

        let result = framework
            .inject_concept_with_metadata("concept-1", vector, metadata)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_framework_builder_with_reservoir_input_size() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .with_reservoir_size(1000)
            .with_reservoir_input_size(512)
            .with_chaos_strength(0.05)
            .build()
            .await
            .unwrap();

        assert_eq!(framework.config.reservoir_input_size, 512);
        assert_eq!(framework.config.reservoir_size, 1000);
        assert_eq!(framework.config.chaos_strength, 0.05);
    }

    #[tokio::test]
    async fn test_get_concept_not_found() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let result = framework.get_concept("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_inject_concept_with_metadata_empty() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let vector = HVec10240::random();
        let metadata = std::collections::HashMap::new();

        framework
            .inject_concept_with_metadata("concept-empty", vector, metadata)
            .await
            .unwrap();

        let concept = framework
            .get_concept("concept-empty")
            .await
            .unwrap()
            .unwrap();
        assert!(concept.metadata.is_empty());
    }
}
