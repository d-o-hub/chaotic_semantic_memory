//! Framework builder and configuration

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ChaoticSemanticFramework;
use crate::error::Result;
use crate::persistence::Persistence;
use crate::singularity::{Singularity, SingularityConfig};

const DEFAULT_MAX_PROBE_TOP_K: usize = 10_000;
const DEFAULT_MAX_CACHED_TOP_K: usize = 100;

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
        }
    }
}

/// Framework statistics
#[derive(Debug, Clone)]
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
}

impl FrameworkBuilder {
    pub(crate) fn new() -> Self {
        Self {
            config: FrameworkConfig::default(),
            db_path: None,
            db_token: None,
            concept_cache_size: SingularityConfig::default().concept_cache_size,
        }
    }

    pub fn with_reservoir_size(mut self, size: usize) -> Self {
        self.config.reservoir_size = size;
        self
    }

    pub fn with_reservoir_input_size(mut self, size: usize) -> Self {
        self.config.reservoir_input_size = size;
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

    pub fn with_concept_cache_size(mut self, size: usize) -> Self {
        self.concept_cache_size = size.max(1);
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

    pub fn with_max_cached_top_k(mut self, max_cached_top_k: usize) -> Self {
        self.config.max_cached_top_k = max_cached_top_k.max(1);
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
            concept_cache_size: self.concept_cache_size,
            max_cached_top_k: self.config.max_cached_top_k,
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
            metrics: Default::default(),
        };

        framework.load_replace().await?;
        Ok(framework)
    }
}
