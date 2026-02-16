use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use ndarray::Array1;
use object_store::path::Path;
use object_store::ObjectStore;
use rayon::prelude::*;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::hyperdim::HVec10240;
use crate::reservoir::EchoStateReservoir;
use crate::singularity::SingularityCore;
use crate::turso::{ensure_schema, ConceptRow, TursoClient};

pub const DEFAULT_RESERVOIR_SIZE: usize = 50_000;
pub const DEFAULT_SPECTRAL_RADIUS: f32 = 1.0;
pub const DEFAULT_CACHE_CAPACITY: usize = 2_048;
pub const DEFAULT_RETRY_ATTEMPTS: usize = 3;
pub const DEFAULT_RETRY_DELAY_MS: u64 = 10;
pub const DEFAULT_RECURRENT_INPUT_WIDTH: usize = 256;
pub const DEFAULT_SYNC_LOCK_RETRIES: usize = 128;
pub const DEFAULT_SYNC_LOCK_YIELD_EVERY: usize = 16;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("database error: {0}")]
    Db(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("object store error: {0}")]
    ObjectStore(String),
    #[error("configuration error: {0}")]
    Config(String),
}

#[derive(Clone)]
pub struct FrameworkBuilder {
    turso_url: Option<String>,
    turso_token: Option<String>,
    reservoir_size: usize,
    spectral_radius: f32,
    cache_capacity: usize,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    retry_attempts: usize,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    retry_delay: Duration,
    recurrent_input_width: usize,
    sync_lock_retries: usize,
    object_store: Option<Arc<dyn ObjectStore>>,
    seeded_concepts: Option<HashMap<String, HVec10240>>,
    #[cfg(not(target_arch = "wasm32"))]
    seeded_client: Option<TursoClient>,
}

pub struct ChaoticSemanticFramework {
    reservoir: Arc<Mutex<EchoStateReservoir>>,
    singularity: Arc<Mutex<SingularityCore>>,
    cache: Arc<Mutex<lru::LruCache<String, HVec10240>>>,
    fallback: Arc<Mutex<HashMap<String, HVec10240>>>,
    pub turso_client: Option<TursoClient>,
    object_store: Option<Arc<dyn ObjectStore>>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    retry_attempts: usize,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    retry_delay: Duration,
    recurrent_input_width: usize,
    sync_lock_retries: usize,
}

fn with_sync_lock<T, R>(
    mutex: &Mutex<T>,
    retries: usize,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    let mut f_opt = Some(f);
    for attempt in 0..retries {
        if let Ok(mut guard) = mutex.try_lock() {
            if let Some(f_run) = f_opt.take() {
                return Some(f_run(&mut guard));
            }
            return None;
        }
        if attempt % DEFAULT_SYNC_LOCK_YIELD_EVERY == 0 {
            std::thread::yield_now();
        } else {
            std::hint::spin_loop();
        }
    }
    None
}

impl ChaoticSemanticFramework {
    pub async fn builder() -> FrameworkBuilder {
        FrameworkBuilder::default()
    }

    pub fn singularity() -> FrameworkBuilder {
        FrameworkBuilder::default()
    }

    pub fn inject_concept(&self, name: &str, hvec: HVec10240) -> f64 {
        let novelty = with_sync_lock(&self.singularity, self.sync_lock_retries, |singularity| {
            singularity.inject_concept(name, hvec)
        });

        let _ = with_sync_lock(&self.fallback, self.sync_lock_retries, |fallback| {
            fallback.insert(name.to_owned(), hvec);
        });

        let _ = with_sync_lock(&self.cache, self.sync_lock_retries, |cache| {
            cache.put(name.to_owned(), hvec);
        });

        novelty.unwrap_or(0.0)
    }

    pub fn singularity_probe(&self, seed: HVec10240, top_k: usize) -> Vec<(String, f64)> {
        let primary = with_sync_lock(&self.singularity, self.sync_lock_retries, |singularity| {
            singularity.probe(seed, top_k)
        })
        .unwrap_or_default();

        let fallback_probe = with_sync_lock(&self.fallback, self.sync_lock_retries, |fallback| {
            let mut items: Vec<(String, f64)> = fallback
                .iter()
                .map(|(name, hv)| (name.clone(), hv.cosine_similarity(&seed)))
                .collect();
            items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            items
        })
        .unwrap_or_default();

        let mut merged = primary;
        for item in fallback_probe {
            if !merged.iter().any(|(name, _)| name == &item.0) {
                merged.push(item);
            }
        }
        merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        merged.truncate(top_k);
        merged
    }

    pub async fn persist_turso(&self, client: &TursoClient) -> Result<(), MemoryError> {
        ensure_schema(client).await?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let concepts: Vec<ConceptRow> = self
                .singularity
                .lock()
                .await
                .concepts
                .iter()
                .map(|(k, v)| ConceptRow::from_pair(k.clone(), *v))
                .collect();

            for row in concepts {
                let mut attempt = 0usize;
                loop {
                    let result = client
                        .execute(
                            "INSERT INTO concepts(name,payload) VALUES (?1,?2) ON CONFLICT(name) DO UPDATE SET payload = excluded.payload",
                            (row.name.clone(), row.bytes.clone()),
                        )
                        .await;
                    match result {
                        Ok(_) => break,
                        Err(err) if attempt + 1 < self.retry_attempts => {
                            attempt += 1;
                            tokio::time::sleep(self.retry_delay).await;
                            let _ = err;
                        }
                        Err(err) => {
                            let snapshot = self.singularity.lock().await.concepts.clone();
                            self.fallback.lock().await.extend(snapshot);
                            return Err(MemoryError::Db(err.to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn restore_turso(client: &TursoClient) -> Result<Self, MemoryError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            ensure_schema(client).await?;
            let rows = client
                .execute("SELECT name,payload FROM concepts", ())
                .await
                .map_err(|e| MemoryError::Db(e.to_string()))?;

            let mut concepts = HashMap::new();
            for row in rows.rows {
                let name: String = row.get(0).map_err(|e| MemoryError::Db(e.to_string()))?;
                let payload: Vec<u8> = row.get(1).map_err(|e| MemoryError::Db(e.to_string()))?;
                if let Some(hv) = HVec10240::from_bytes(&payload) {
                    concepts.insert(name, hv);
                }
            }

            return FrameworkBuilder::default()
                .with_seeded_concepts(concepts)
                .with_seeded_client(client.clone())
                .build()
                .await;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = client;
            FrameworkBuilder::default().build().await
        }
    }

    pub async fn retrieve_parallel(&self, probe: HVec10240, top_k: usize) -> Vec<(String, f64)> {
        let concepts = self.singularity.lock().await.concepts.clone();
        let mut out: Vec<(String, f64)> = concepts
            .par_iter()
            .map(|(k, v)| (k.clone(), v.cosine_similarity(&probe)))
            .collect();
        out.par_sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(top_k);
        out
    }

    pub async fn recurrent_step(&self) -> f64 {
        let mut reservoir = self.reservoir.lock().await;
        reservoir.recurrent_step(&Array1::ones(self.recurrent_input_width))
    }

    pub async fn checkpoint_object_store(&self, key: &str) -> Result<(), MemoryError> {
        if let Some(store) = &self.object_store {
            let concepts = self.singularity.lock().await.concepts.clone();
            let rows: Vec<ConceptRow> = concepts
                .into_iter()
                .map(|(k, v)| ConceptRow::from_pair(k, v))
                .collect();
            let bytes =
                simd_json::to_vec(&rows).map_err(|e| MemoryError::Serialization(e.to_string()))?;
            store
                .put(&Path::from(key), bytes.into())
                .await
                .map_err(|e| MemoryError::ObjectStore(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self {
            turso_url: None,
            turso_token: None,
            reservoir_size: DEFAULT_RESERVOIR_SIZE,
            spectral_radius: DEFAULT_SPECTRAL_RADIUS,
            cache_capacity: DEFAULT_CACHE_CAPACITY,
            retry_attempts: DEFAULT_RETRY_ATTEMPTS,
            retry_delay: Duration::from_millis(DEFAULT_RETRY_DELAY_MS),
            recurrent_input_width: DEFAULT_RECURRENT_INPUT_WIDTH,
            sync_lock_retries: DEFAULT_SYNC_LOCK_RETRIES,
            object_store: None,
            seeded_concepts: None,
            #[cfg(not(target_arch = "wasm32"))]
            seeded_client: None,
        }
    }
}

impl FrameworkBuilder {
    pub fn with_turso(mut self, url: impl Into<String>, token: impl Into<String>) -> Self {
        self.turso_url = Some(url.into());
        self.turso_token = Some(token.into());
        self
    }

    pub fn with_reservoir_size(mut self, size: usize) -> Self {
        self.reservoir_size = size;
        self
    }

    pub fn with_spectral_radius(mut self, radius: f32) -> Self {
        self.spectral_radius = radius;
        self
    }

    pub fn with_cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity;
        self
    }

    pub fn with_retry_policy(mut self, attempts: usize, delay: Duration) -> Self {
        self.retry_attempts = attempts;
        self.retry_delay = delay;
        self
    }

    pub fn with_recurrent_input_width(mut self, width: usize) -> Self {
        self.recurrent_input_width = width;
        self
    }

    pub fn with_sync_lock_retries(mut self, retries: usize) -> Self {
        self.sync_lock_retries = retries;
        self
    }

    pub fn with_object_store(mut self, store: Arc<dyn ObjectStore>) -> Self {
        self.object_store = Some(store);
        self
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn with_seeded_concepts(mut self, concepts: HashMap<String, HVec10240>) -> Self {
        self.seeded_concepts = Some(concepts);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn with_seeded_client(mut self, client: TursoClient) -> Self {
        self.seeded_client = Some(client);
        self
    }

    pub async fn build(self) -> Result<ChaoticSemanticFramework, MemoryError> {
        let cache_capacity = NonZeroUsize::new(self.cache_capacity)
            .ok_or_else(|| MemoryError::Config("cache capacity must be non-zero".to_string()))?;
        if self.retry_attempts == 0 {
            return Err(MemoryError::Config(
                "retry attempts must be greater than zero".to_string(),
            ));
        }
        if self.sync_lock_retries == 0 {
            return Err(MemoryError::Config(
                "sync lock retries must be greater than zero".to_string(),
            ));
        }

        let mut singularity = SingularityCore::default();
        if let Some(concepts) = self.seeded_concepts {
            singularity.concepts = concepts;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let client = match self.seeded_client {
            Some(client) => Some(client),
            None => match (self.turso_url, self.turso_token) {
                (Some(url), Some(token)) => Some(
                    turso_client::Client::new(url, token)
                        .map_err(|e| MemoryError::Db(e.to_string()))?,
                ),
                _ => None,
            },
        };
        #[cfg(target_arch = "wasm32")]
        let client = None;

        Ok(ChaoticSemanticFramework {
            reservoir: Arc::new(Mutex::new(EchoStateReservoir::new(
                self.reservoir_size,
                self.spectral_radius,
            ))),
            singularity: Arc::new(Mutex::new(singularity)),
            cache: Arc::new(Mutex::new(lru::LruCache::new(cache_capacity))),
            fallback: Arc::new(Mutex::new(HashMap::new())),
            turso_client: client,
            object_store: self.object_store,
            retry_attempts: self.retry_attempts,
            retry_delay: self.retry_delay,
            recurrent_input_width: self.recurrent_input_width,
            sync_lock_retries: self.sync_lock_retries,
        })
    }
}
