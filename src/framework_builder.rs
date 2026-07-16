//! Framework builder and configuration

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

use crate::ChaoticSemanticFramework;
use crate::framework_events::build_event_sender;
use crate::framework_events_ce::EventEmitter;
#[cfg(feature = "persistence")]
use crate::persistence::Persistence;
use crate::singularity::{Singularity, SingularityConfig};
use csm_core::error::Result;
use csm_core::reservoir::Reservoir;

const DEFAULT_MAX_PROBE_TOP_K: usize = 10_000;
const DEFAULT_MAX_CACHED_TOP_K: usize = 100;
const DEFAULT_MAX_BATCH_SIZE: usize = 1000;
const DEFAULT_MAX_SEQUENCE_LENGTH: usize = 1024;

/// Runtime configuration for [`ChaoticSemanticFramework`], tuned via [`FrameworkBuilder`].
#[derive(Clone, Debug)]
pub struct FrameworkConfig {
    /// Reservoir node count (default: `50_000`, must be `> 0`).
    pub reservoir_size: usize,
    /// Input width per sequence step (default: `10_240`, must be `> 0`).
    pub reservoir_input_size: usize,
    /// Chaotic noise magnitude (default: `0.1`, recommended: `0.0..=1.0`).
    pub chaos_strength: f32,
    /// Enables persistence setup at build time (default: `true`).
    pub enable_persistence: bool,
    /// Maximum concept count before oldest-concept eviction (default: `None`).
    pub max_concepts: Option<usize>,
    /// Maximum outbound associations per concept (default: `None`).
    pub max_associations_per_concept: Option<usize>,
    /// Remote libSQL pool size (default: `10`, coerced to `>= 1`).
    pub connection_pool_size: usize,
    /// Upper bound for `top_k` in probes (default: `10_000`, coerced to `>= 1`).
    pub max_probe_top_k: usize,
    /// Optional metadata size limit in bytes per concept (default: `None`).
    pub max_metadata_bytes: Option<usize>,
    /// Maximum top_k for cache eligibility (default: `100`).
    /// Queries with top_k > this value bypass the cache.
    pub max_cached_top_k: usize,
    /// Maximum items in a batch operation (default: `1000`).
    pub max_batch_size: usize,
    /// Maximum steps in a temporal sequence (default: `1024`).
    pub max_sequence_length: usize,
    /// ANN index backend (default: `BruteForce`).
    pub index_backend: crate::index::IndexBackend,
    /// Cosine similarity threshold for pattern recognition events (default: `0.9`).
    pub pattern_recognition_threshold: f64,
    /// Advanced TTL and decay configuration.
    pub ttl_config: crate::framework_ttl_advanced::TtlConfig,
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
            max_cached_top_k: DEFAULT_MAX_CACHED_TOP_K,
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            max_sequence_length: DEFAULT_MAX_SEQUENCE_LENGTH,
            index_backend: crate::index::IndexBackend::BruteForce,
            pattern_recognition_threshold: 0.9,
            ttl_config: crate::framework_ttl_advanced::TtlConfig::default(),
        }
    }
}

/// Framework statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct FrameworkStats {
    pub concept_count: usize,
    /// Database size in bytes. `None` if persistence is disabled or size unavailable.
    pub db_size_bytes: Option<u64>,
}

/// Builder for ChaoticSemanticFramework
pub struct FrameworkBuilder {
    pub(crate) config: FrameworkConfig,
    pub(crate) db_path: Option<String>,
    pub(crate) db_token: Option<String>,
    pub(crate) concept_cache_size: usize,
    pub(crate) version_retention: usize,
    pub(crate) namespace: String,
    pub(crate) embedding_provider: Option<Arc<dyn crate::embedding::EmbeddingProvider>>,
    pub(crate) emitters: Vec<Arc<dyn EventEmitter>>,
}

impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self {
            config: FrameworkConfig::default(),
            db_path: None,
            db_token: None,
            concept_cache_size: 1000,
            version_retention: 10,
            namespace: "_default".to_string(),
            embedding_provider: None,
            emitters: Vec::new(),
        }
    }
}

impl FrameworkBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        let ns = ns.into();
        if let Err(e) = ChaoticSemanticFramework::validate_namespace(&ns) {
            warn!(
                "invalid namespace supplied to builder ({}), keeping default",
                e
            );
        } else {
            self.namespace = ns;
        }
        self
    }

    pub fn with_reservoir_size(mut self, mut size: usize) -> Self {
        if size > crate::framework_validation::MAX_RESERVOIR_SIZE_LIMIT {
            warn!(
                "reservoir size {} exceeds limit {}, clamping",
                size,
                crate::framework_validation::MAX_RESERVOIR_SIZE_LIMIT
            );
            size = crate::framework_validation::MAX_RESERVOIR_SIZE_LIMIT;
        }
        self.config.reservoir_size = size;
        self
    }

    pub fn with_reservoir_input_size(mut self, mut size: usize) -> Self {
        if size > crate::framework_validation::MAX_RESERVOIR_SIZE_LIMIT {
            warn!(
                "reservoir input size {} exceeds limit {}, clamping",
                size,
                crate::framework_validation::MAX_RESERVOIR_SIZE_LIMIT
            );
            size = crate::framework_validation::MAX_RESERVOIR_SIZE_LIMIT;
        }
        self.config.reservoir_input_size = size;
        self
    }

    pub const fn with_chaos_strength(mut self, strength: f32) -> Self {
        self.config.chaos_strength = strength;
        self
    }

    pub fn with_max_concepts(mut self, mut max_concepts: usize) -> Self {
        if max_concepts > crate::framework_validation::MAX_STORE_CAPACITY_LIMIT {
            warn!(
                "max concepts {} exceeds limit {}, clamping",
                max_concepts,
                crate::framework_validation::MAX_STORE_CAPACITY_LIMIT
            );
            max_concepts = crate::framework_validation::MAX_STORE_CAPACITY_LIMIT;
        }
        self.config.max_concepts = Some(max_concepts);
        self
    }

    pub fn with_max_associations_per_concept(mut self, mut max_associations: usize) -> Self {
        let limit = crate::framework_validation::MAX_ASSOCIATIONS_PER_CONCEPT_LIMIT;
        if max_associations > limit {
            warn!(
                "max associations per concept {} exceeds limit {}, clamping",
                max_associations, limit
            );
            max_associations = limit;
        }
        self.config.max_associations_per_concept = Some(max_associations);
        self
    }

    pub fn with_concept_cache_size(mut self, size: usize) -> Self {
        self.concept_cache_size =
            size.clamp(1, crate::framework_validation::MAX_CONCEPT_CACHE_LIMIT);
        self
    }

    /// Configure the connection pool size for remote Turso databases.
    ///
    /// Only available when the `persistence` feature is enabled.
    #[cfg(feature = "persistence")]
    pub fn with_connection_pool_size(mut self, pool_size: usize) -> Self {
        self.config.connection_pool_size =
            pool_size.clamp(1, crate::framework_validation::MAX_CONNECTION_POOL_LIMIT);
        self
    }

    pub fn with_max_probe_top_k(mut self, max_probe_top_k: usize) -> Self {
        self.config.max_probe_top_k =
            max_probe_top_k.clamp(1, crate::framework_validation::MAX_TOP_K_LIMIT);
        self
    }

    pub const fn with_max_metadata_bytes(mut self, max_metadata_bytes: usize) -> Self {
        let limit = crate::framework_validation::MAX_METADATA_BYTES_LIMIT;
        self.config.max_metadata_bytes = Some(if max_metadata_bytes > limit {
            limit
        } else {
            max_metadata_bytes
        });
        self
    }

    pub fn with_max_cached_top_k(mut self, max_cached_top_k: usize) -> Self {
        self.config.max_cached_top_k =
            max_cached_top_k.clamp(1, crate::framework_validation::MAX_TOP_K_LIMIT);
        self
    }

    pub fn with_max_batch_size(mut self, max_batch_size: usize) -> Self {
        self.config.max_batch_size =
            max_batch_size.clamp(1, crate::framework_validation::MAX_BATCH_SIZE_LIMIT);
        self
    }

    pub fn with_max_sequence_length(mut self, max_sequence_length: usize) -> Self {
        self.config.max_sequence_length =
            max_sequence_length.clamp(1, crate::framework_validation::MAX_SEQUENCE_LENGTH_LIMIT);
        self
    }

    /// Set the cosine similarity threshold for pattern recognition events.
    pub fn with_pattern_recognition_threshold(mut self, threshold: f64) -> Self {
        if threshold.is_finite() {
            self.config.pattern_recognition_threshold = threshold.clamp(0.0, 1.0);
        } else {
            self.config.pattern_recognition_threshold = 0.0;
        }
        self
    }

    /// Add an event emitter to the framework.
    pub fn with_emitter(mut self, emitter: Arc<dyn EventEmitter>) -> Self {
        self.emitters.push(emitter);
        self
    }

    pub const fn with_index_backend(mut self, backend: crate::index::IndexBackend) -> Self {
        self.config.index_backend = backend;
        self
    }

    pub fn with_ttl_config(mut self, config: crate::framework_ttl_advanced::TtlConfig) -> Self {
        self.config.ttl_config = config;
        self
    }

    /// Keep the last N historical versions per concept in persistence.
    ///
    /// Values less than 1 are coerced to 1. Default is 10.
    pub fn with_version_retention(mut self, retention: usize) -> Self {
        self.version_retention =
            retention.clamp(1, crate::framework_validation::MAX_VERSION_RETENTION_LIMIT);
        self
    }

    /// Configure a local SQLite database for persistence.
    ///
    /// Only available when the `persistence` feature is enabled (ADR-0094:
    /// disabled capabilities are cfg-absent, never silently ignored).
    #[cfg(feature = "persistence")]
    pub fn with_local_db(mut self, path: impl Into<String>) -> Self {
        self.db_path = Some(path.into());
        self.db_token = None;
        self
    }

    /// Configure a remote Turso database for persistence.
    ///
    /// Only available when the `persistence` feature is enabled (ADR-0094).
    #[cfg(feature = "persistence")]
    pub fn with_turso(mut self, url: impl Into<String>, token: impl Into<String>) -> Self {
        self.db_path = Some(url.into());
        self.db_token = Some(token.into());
        self
    }

    /// Disable persistence even when the feature is enabled.
    #[cfg(feature = "persistence")]
    pub const fn without_persistence(mut self) -> Self {
        self.config.enable_persistence = false;
        self
    }

    /// No-op when the `persistence` feature is disabled (already unavailable).
    #[cfg(not(feature = "persistence"))]
    pub const fn without_persistence(self) -> Self {
        self
    }

    /// Configure an external embedding provider.
    pub fn with_embedding_provider<P: crate::embedding::EmbeddingProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.embedding_provider = Some(Arc::new(provider));
        self
    }

    /// Configure an external embedding provider using an existing Arc'd trait object.
    pub fn with_embedding_provider_arc(
        mut self,
        provider: Arc<dyn crate::embedding::EmbeddingProvider>,
    ) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    pub async fn build(self) -> Result<ChaoticSemanticFramework> {
        Reservoir::validate_params(
            self.config.reservoir_size,
            self.config.reservoir_input_size,
            self.config.chaos_strength,
        )?;
        // Fail closed on invalid ANN params (HNSW m, LSH tables, etc.) at build
        // time rather than panicking on first namespace creation (ADR-0093).
        crate::index::validate_index_backend(&self.config.index_backend)?;
        let metrics = Arc::new(crate::framework_metrics::FrameworkMetrics::default());

        let singularity = Arc::new(RwLock::new(Singularity::with_config_backend_and_metrics(
            SingularityConfig {
                max_concepts: self.config.max_concepts,
                max_associations_per_concept: self.config.max_associations_per_concept,
                concept_cache_size: self.concept_cache_size,
                index_backend: self.config.index_backend.clone(),
                max_cached_top_k: self.config.max_cached_top_k,
            },
            self.config.index_backend.clone(),
            Arc::clone(&metrics.cache_metrics),
        )));

        #[cfg(feature = "persistence")]
        let persistence = if self.config.enable_persistence {
            if let Some(path) = self.db_path {
                let persist = if let Some(token) = self.db_token {
                    Persistence::new_turso_with_pool_and_retention(
                        &path,
                        &token,
                        self.config.connection_pool_size,
                        self.version_retention,
                    )
                    .await?
                } else {
                    Persistence::new_local_with_retention(&path, self.version_retention).await?
                };
                Some(Arc::new(persist))
            } else {
                None
            }
        } else {
            None
        };

        #[cfg(not(feature = "persistence"))]
        let persistence: Option<Arc<crate::persistence::Persistence>> = None;

        let provider = self
            .embedding_provider
            .unwrap_or_else(|| Arc::new(crate::embedding::HdcTextProvider::new()));

        let projection = if provider.name() == "hdc-text" {
            crate::embedding::Projection::empty()
        } else {
            crate::embedding::Projection::new(&crate::embedding::ProjectionConfig {
                native_dim: provider.native_dim(),
                ..Default::default()
            })
        };

        // `mut` only used when attaching ttl_cleanup (non-wasm).
        #[allow(unused_mut)]
        let mut framework = ChaoticSemanticFramework {
            singularity,
            persistence,
            reservoir: Arc::new(RwLock::new(None)),
            config: self.config,
            metrics,
            event_sender: build_event_sender(),
            emitters: self.emitters,
            namespace: Arc::new(RwLock::new(self.namespace)),
            embedding_provider: provider,
            projection: Arc::new(projection),
            #[cfg(not(target_arch = "wasm32"))]
            ttl_cleanup: None,
        };

        framework.load_replace().await?;

        // Start owned background cleanup when configured (ADR-0093 lifecycle).
        #[cfg(not(target_arch = "wasm32"))]
        {
            let interval = framework.config.ttl_config.cleanup_interval_seconds;
            if interval > 0 {
                let cancel = crate::framework_ttl::TtlCleanupControl::new_cancel();
                let cancel_task = Arc::clone(&cancel);
                let fw_task = framework.clone();
                let handle = tokio::spawn(async move {
                    let mut timer =
                        tokio::time::interval(tokio::time::Duration::from_secs(interval));
                    // Skip the immediate first tick so build() returns before first purge.
                    timer.tick().await;
                    loop {
                        if cancel_task.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        timer.tick().await;
                        if cancel_task.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        if let Err(e) = fw_task.purge_expired().await {
                            tracing::error!(error = %e, "background cleanup failed");
                        }
                    }
                });
                framework.ttl_cleanup = Some(Arc::new(
                    crate::framework_ttl::TtlCleanupControl::new(cancel, handle),
                ));
            }
        }

        Ok(framework)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_max_associations_per_concept_clamping() {
        let limit = crate::framework_validation::MAX_ASSOCIATIONS_PER_CONCEPT_LIMIT;

        // Above limit
        let builder = FrameworkBuilder::new().with_max_associations_per_concept(limit + 1);
        assert_eq!(builder.config.max_associations_per_concept, Some(limit));

        // At limit
        let builder = FrameworkBuilder::new().with_max_associations_per_concept(limit);
        assert_eq!(builder.config.max_associations_per_concept, Some(limit));

        // Below limit
        let builder = FrameworkBuilder::new().with_max_associations_per_concept(limit - 1);
        assert_eq!(builder.config.max_associations_per_concept, Some(limit - 1));
    }

    #[tokio::test]
    async fn build_default_bruteforce_ok() {
        let fw = FrameworkBuilder::new()
            .without_persistence()
            .build()
            .await
            .expect("default BruteForce backend must build");
        fw.inject_concept("c1", csm_core::HVec10240::random())
            .await
            .expect("inject on default backend");
    }
}
