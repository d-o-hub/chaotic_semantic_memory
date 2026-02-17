//! Main framework integrating all components

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{instrument, warn};

use crate::error::Result;
use crate::hyperdim::HVec10240;
use crate::persistence::Persistence;
use crate::reservoir::ChaoticReservoir;
use crate::singularity::{Concept, ConceptBuilder, Singularity, SingularityConfig};

const DEFAULT_MAX_PROBE_TOP_K: usize = 10_000;

/// Main framework for chaotic semantic memory
pub struct ChaoticSemanticFramework {
    pub(crate) singularity: Arc<RwLock<Singularity>>,
    pub(crate) persistence: Option<Arc<Persistence>>,
    pub(crate) reservoir: Arc<RwLock<Option<ChaoticReservoir>>>,
    pub(crate) config: FrameworkConfig,
    pub(crate) metrics: Arc<FrameworkMetrics>,
}

#[derive(Clone, Debug)]
pub struct FrameworkConfig {
    pub reservoir_size: usize,
    pub reservoir_input_size: usize,
    pub chaos_strength: f32,
    pub enable_persistence: bool,
    pub max_concepts: Option<usize>,
    pub max_associations_per_concept: Option<usize>,
    pub connection_pool_size: usize,
    pub max_probe_top_k: usize,
    pub max_metadata_bytes: Option<usize>,
}

impl Default for FrameworkConfig {
    fn default() -> Self {
        Self {
            reservoir_size: 50000,
            reservoir_input_size: 10240,
            chaos_strength: 0.1,
            enable_persistence: true,
            max_concepts: None,
            max_associations_per_concept: None,
            connection_pool_size: 10,
            max_probe_top_k: DEFAULT_MAX_PROBE_TOP_K,
            max_metadata_bytes: None,
        }
    }
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
        }
    }
}

impl ChaoticSemanticFramework {
    /// Create a new framework builder
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

        let r = reservoir.as_mut().expect("reservoir initialized above");
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

            let mut sing = self.singularity.write().await;
            sing.clear();
            for concept in concepts {
                self.validate_concept(&concept)?;
                sing.inject(concept)?;
            }

            // Associations are loaded after concept insertions.
            let concept_ids = sing.concept_ids();
            for concept_id in concept_ids {
                let links = persistence.load_associations(&concept_id).await?;
                for (to_id, strength) in links {
                    if let Err(error) = sing.associate(&concept_id, &to_id, strength) {
                        warn!(
                            from_id = %concept_id,
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
            let mut sing = self.singularity.write().await;
            for concept in concepts {
                self.validate_concept(&concept)?;
                sing.inject(concept)?;
            }

            let concept_ids = sing.concept_ids();
            for concept_id in concept_ids {
                let links = persistence.load_associations(&concept_id).await?;
                for (to_id, strength) in links {
                    if let Err(error) = sing.associate(&concept_id, &to_id, strength) {
                        warn!(
                            from_id = %concept_id,
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

    pub fn metrics_snapshot(&self) -> FrameworkMetricsSnapshot {
        self.metrics.snapshot()
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

/// Framework statistics
#[derive(Debug, Clone)]
pub struct FrameworkStats {
    pub concept_count: usize,
    pub db_size_bytes: u64,
}

/// Builder for ChaoticSemanticFramework
pub struct FrameworkBuilder {
    config: FrameworkConfig,
    db_path: Option<String>,
    db_token: Option<String>,
}

impl FrameworkBuilder {
    fn new() -> Self {
        Self {
            config: FrameworkConfig::default(),
            db_path: None,
            db_token: None,
        }
    }

    pub fn with_reservoir_size(mut self, size: usize) -> Self {
        self.config.reservoir_size = size;
        self
    }

    pub fn with_chaos_strength(mut self, strength: f32) -> Self {
        self.config.chaos_strength = strength;
        self
    }

    pub fn with_max_concepts(mut self, max_concepts: usize) -> Self {
        self.config.max_concepts = Some(max_concepts);
        self
    }

    pub fn with_max_associations_per_concept(mut self, max_associations: usize) -> Self {
        self.config.max_associations_per_concept = Some(max_associations);
        self
    }

    pub fn with_turso(mut self, url: impl Into<String>, token: impl Into<String>) -> Self {
        self.db_path = Some(url.into());
        self.db_token = Some(token.into());
        self
    }

    pub fn with_connection_pool_size(mut self, pool_size: usize) -> Self {
        self.config.connection_pool_size = pool_size.max(1);
        self
    }

    pub fn with_max_probe_top_k(mut self, max_probe_top_k: usize) -> Self {
        self.config.max_probe_top_k = max_probe_top_k.max(1);
        self
    }

    pub fn with_max_metadata_bytes(mut self, max_metadata_bytes: usize) -> Self {
        self.config.max_metadata_bytes = Some(max_metadata_bytes);
        self
    }

    pub fn with_local_db(mut self, path: impl Into<String>) -> Self {
        self.db_path = Some(path.into());
        self.db_token = None;
        self
    }

    pub fn without_persistence(mut self) -> Self {
        self.config.enable_persistence = false;
        self
    }

    pub async fn build(self) -> Result<ChaoticSemanticFramework> {
        let singularity = Arc::new(RwLock::new(Singularity::with_config(SingularityConfig {
            max_concepts: self.config.max_concepts,
            max_associations_per_concept: self.config.max_associations_per_concept,
            concept_cache_size: 1000,
        })));

        let persistence = if self.config.enable_persistence {
            if let Some(path) = self.db_path {
                let persist = if let Some(token) = self.db_token {
                    Persistence::new_turso_with_pool(
                        &path,
                        &token,
                        self.config.connection_pool_size,
                    )
                    .await?
                } else {
                    Persistence::new_local(&path).await?
                };
                Some(Arc::new(persist))
            } else {
                None
            }
        } else {
            None
        };

        let framework = ChaoticSemanticFramework {
            singularity,
            persistence,
            reservoir: Arc::new(RwLock::new(None)),
            config: self.config,
            metrics: Arc::new(FrameworkMetrics::default()),
        };

        framework.load_replace().await?;
        Ok(framework)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_framework_creation() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let stats = framework.stats().await.unwrap();
        assert_eq!(stats.concept_count, 0);
    }

    #[tokio::test]
    async fn test_concept_lifecycle() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        framework
            .inject_concept("a", HVec10240::random())
            .await
            .unwrap();
        framework
            .inject_concept("b", HVec10240::random())
            .await
            .unwrap();
        framework.associate("a", "b", 0.8).await.unwrap();

        let associations = framework.get_associations("a").await.unwrap();
        assert_eq!(associations.len(), 1);

        framework.delete_concept("b").await.unwrap();
        let associations = framework.get_associations("a").await.unwrap();
        assert!(associations.is_empty());
    }
}
