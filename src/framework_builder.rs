use crate::embedding::projection::{Projection, ProjectionConfig};
use crate::embedding::hdc_text::HdcTextProvider;
/// Framework builder and configuration

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ChaoticSemanticFramework;
use crate::error::Result;
use crate::framework_events::build_event_sender;
#[cfg(feature = "persistence")]
use crate::persistence::Persistence;
use crate::reservoir::Reservoir;
use crate::singularity::{Singularity, SingularityConfig};
use crate::hyperdim::{HVec10240, Hypervector};

const DEFAULT_MAX_PROBE_TOP_K: usize = 10_000;
const DEFAULT_MAX_CACHED_TOP_K: usize = 100;
const DEFAULT_MAX_BATCH_SIZE: usize = 1000;
const DEFAULT_MAX_SEQUENCE_LENGTH: usize = 1024;

/// Runtime configuration for ChaoticSemanticFramework, tuned via FrameworkBuilder.
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
    pub max_cached_top_k: usize,
    pub max_batch_size: usize,
    pub max_sequence_length: usize,
    pub index_backend: crate::index::IndexBackend,
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
        }
    }
}

/// Framework statistics
#[derive(Debug, Clone)]
pub struct FrameworkStats {
    pub concept_count: usize,
    pub db_size_bytes: Option<u64>,
}

/// Builder for ChaoticSemanticFramework
pub struct FrameworkBuilder<H: Hypervector = HVec10240> {
    pub(crate) config: FrameworkConfig,
    pub(crate) db_path: Option<String>,
    pub(crate) db_token: Option<String>,
    pub(crate) concept_cache_size: usize,
    pub(crate) version_retention: usize,
    pub(crate) namespace: String,
    pub(crate) embedding_provider: Option<Arc<dyn crate::embedding::EmbeddingProvider>>,
    _phantom: std::marker::PhantomData<H>,
}

impl<H: Hypervector> Default for FrameworkBuilder<H> {
    fn default() -> Self {
        Self {
            config: FrameworkConfig::default(),
            db_path: None,
            db_token: None,
            concept_cache_size: 1000,
            version_retention: 10,
            namespace: "_default".to_string(),
            embedding_provider: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<H: Hypervector + 'static> FrameworkBuilder<H> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = ns.into();
        self
    }

    pub const fn with_reservoir_size(mut self, size: usize) -> Self {
        self.config.reservoir_size = size;
        self
    }

    pub const fn with_reservoir_input_size(mut self, size: usize) -> Self {
        self.config.reservoir_input_size = size;
        self
    }

    pub const fn with_chaos_strength(mut self, strength: f32) -> Self {
        self.config.chaos_strength = strength;
        self
    }

    pub const fn with_max_concepts(mut self, max_concepts: usize) -> Self {
        self.config.max_concepts = Some(max_concepts);
        self
    }

    pub const fn with_max_associations_per_concept(mut self, max_associations: usize) -> Self {
        self.config.max_associations_per_concept = Some(max_associations);
        self
    }

    pub fn with_concept_cache_size(mut self, size: usize) -> Self {
        self.concept_cache_size = size.max(1);
        self
    }

    #[cfg(feature = "persistence")]
    pub fn with_connection_pool_size(mut self, pool_size: usize) -> Self {
        self.config.connection_pool_size = pool_size.max(1);
        self
    }

    pub fn with_max_probe_top_k(mut self, max_probe_top_k: usize) -> Self {
        self.config.max_probe_top_k = max_probe_top_k.max(1);
        self
    }

    pub const fn with_max_metadata_bytes(mut self, max_metadata_bytes: usize) -> Self {
        self.config.max_metadata_bytes = Some(max_metadata_bytes);
        self
    }

    pub fn with_max_cached_top_k(mut self, max_cached_top_k: usize) -> Self {
        self.config.max_cached_top_k = max_cached_top_k.max(1);
        self
    }

    pub fn with_max_batch_size(mut self, max_batch_size: usize) -> Self {
        self.config.max_batch_size = max_batch_size.max(1);
        self
    }

    pub fn with_max_sequence_length(mut self, max_sequence_length: usize) -> Self {
        self.config.max_sequence_length = max_sequence_length.max(1);
        self
    }

    pub const fn with_index_backend(mut self, backend: crate::index::IndexBackend) -> Self {
        self.config.index_backend = backend;
        self
    }

    pub fn with_version_retention(mut self, retention: usize) -> Self {
        self.version_retention = retention.max(1);
        self
    }

    #[cfg(feature = "persistence")]
    pub fn with_local_db(mut self, path: impl Into<String>) -> Self {
        self.db_path = Some(path.into());
        self.db_token = None;
        self
    }

    #[cfg(feature = "persistence")]
    pub fn with_turso(mut self, url: impl Into<String>, token: impl Into<String>) -> Self {
        self.db_path = Some(url.into());
        self.db_token = Some(token.into());
        self
    }

    #[cfg(feature = "persistence")]
    pub const fn without_persistence(mut self) -> Self {
        self.config.enable_persistence = false;
        self
    }

    #[cfg(not(feature = "persistence"))]
    pub fn without_persistence(self) -> Self {
        self
    }

    pub fn with_embedding_provider<P: crate::embedding::EmbeddingProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.embedding_provider = Some(Arc::new(provider));
        self
    }

    pub fn with_embedding_provider_arc(
        mut self,
        provider: Arc<dyn crate::embedding::EmbeddingProvider>,
    ) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    pub async fn build(self) -> Result<ChaoticSemanticFramework<H>> {
        Reservoir::validate_params(
            self.config.reservoir_size,
            self.config.reservoir_input_size,
            self.config.chaos_strength,
        )?;
        let singularity = Arc::new(RwLock::new(Singularity::<H>::with_config_and_backend(
            SingularityConfig {
                max_concepts: self.config.max_concepts,
                max_associations_per_concept: self.config.max_associations_per_concept,
                concept_cache_size: self.concept_cache_size,
                index_backend: self.config.index_backend.clone(),
                max_cached_top_k: self.config.max_cached_top_k,
            },
            self.config.index_backend.clone(),
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
        let persistence: Option<Arc<Persistence>> = None;

        let provider = self
            .embedding_provider
            .unwrap_or_else(|| Arc::new(HdcTextProvider::new()));

        let projection = if provider.name() == "hdc" {
            Projection::empty()
        } else {
            Projection::new(&ProjectionConfig {
                native_dim: provider.dimension(),
                ..Default::default()
            })
        };

        let framework = ChaoticSemanticFramework {
            singularity,
            persistence,
            reservoir: Arc::new(RwLock::new(None)),
            config: self.config,
            metrics: Arc::new(crate::framework_metrics::FrameworkMetrics::default()),
            event_sender: build_event_sender(),
            namespace: Arc::new(RwLock::new(self.namespace)),
            embedding_provider: provider,
            projection: Arc::new(projection),
        };

        framework.load_replace().await?;
        Ok(framework)
    }
}
