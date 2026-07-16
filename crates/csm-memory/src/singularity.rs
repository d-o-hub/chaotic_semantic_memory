//! Core concept storage and indexing engine
#![allow(clippy::cast_precision_loss)] // u64 timestamps → f32 for TTL decay math is intentional

pub use crate::singularity_types::*;

use crate::index::{AnnIndex, IndexBackend, IndexStats};
use crate::singularity_cache::{CacheMetrics, CacheMetricsSnapshot};
use crate::singularity_retrieval::RetrievalConfig;
use crate::singularity_state::NamespaceState;
use csm_core::error::{MemoryError, Result};
use csm_core::hyperdim::Hypervector;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

pub struct Singularity<H: Hypervector = csm_core::hyperdim::HVec10240> {
    pub config: SingularityConfig,
    pub namespaces: HashMap<String, NamespaceState<H>>,
    pub(crate) _retrieval_config: RetrievalConfig,
    pub cache_metrics: Arc<CacheMetrics>,
}

/// Type alias for binary (quantized) hypervector-backed singularity engine.
#[allow(dead_code)]
pub type BinarySingularity = Singularity<csm_core::BHVec10240>;

impl<H: Hypervector + 'static> Singularity<H> {
    pub fn new(config: SingularityConfig) -> Self {
        Self::new_with_metrics(config, Arc::new(CacheMetrics::default()))
    }

    pub fn new_with_metrics(config: SingularityConfig, cache_metrics: Arc<CacheMetrics>) -> Self {
        Self {
            config,
            namespaces: HashMap::new(),
            _retrieval_config: RetrievalConfig::default(),
            cache_metrics,
        }
    }

    pub fn with_config(config: SingularityConfig) -> Self {
        Self::new(config)
    }

    pub fn with_config_and_backend(config: SingularityConfig, backend: IndexBackend) -> Self {
        let mut cfg = config;
        cfg.index_backend = backend;
        Self::new(cfg)
    }

    pub fn with_config_backend_and_metrics(
        config: SingularityConfig,
        backend: IndexBackend,
        cache_metrics: Arc<CacheMetrics>,
    ) -> Self {
        let mut cfg = config;
        cfg.index_backend = backend;
        Self::new_with_metrics(cfg, cache_metrics)
    }

    fn create_index(&self) -> Result<Box<dyn AnnIndex<H>>> {
        crate::index::create_index(&self.config.index_backend)
    }

    pub fn get_namespace(&self, ns: &str) -> Option<&NamespaceState<H>> {
        self.namespaces.get(ns)
    }

    /// Ensure a namespace exists, creating its ANN index if needed.
    ///
    /// Returns `Err(MemoryError::InvalidInput)` when the configured ANN backend
    /// cannot be constructed (e.g. invalid HNSW/LSH parameters).
    pub fn ensure_namespace(&mut self, ns: &str) -> Result<&mut NamespaceState<H>> {
        if !self.namespaces.contains_key(ns) {
            let index = self.create_index()?;
            self.namespaces.insert(
                ns.to_string(),
                NamespaceState::new(&self.config, index, Arc::clone(&self.cache_metrics)),
            );
        }
        // Key was present or just inserted; absence would be a logic bug.
        self.namespaces
            .get_mut(ns)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Namespace".to_string(),
                id: ns.to_string(),
            })
    }

    /// Mutable access to a namespace, creating it if absent.
    ///
    /// **Breaking change (0.3.x → next):** returns `Result` instead of a bare
    /// `&mut NamespaceState`. Prefer [`Self::ensure_namespace`]. Invalid ANN
    /// backend configuration propagates as `MemoryError::InvalidInput` rather
    /// than panicking. Migration: use `?` or handle the `Result` at call sites.
    pub fn get_namespace_mut(&mut self, ns: &str) -> Result<&mut NamespaceState<H>> {
        self.ensure_namespace(ns)
    }

    #[instrument(skip(self, concept))]
    pub fn inject(&mut self, ns: &str, concept: Concept<H>) -> Result<()> {
        self.evict_oldest_if_needed(ns);
        let id = concept.id.clone();
        let vector = concept.vector;

        let ns_state = self.ensure_namespace(ns)?;

        // Update ANN index
        ns_state.index.insert(id.clone(), &vector)?;

        if let Some(_old) = ns_state.concepts.insert(id.clone(), concept) {
            if let Some(pos) = ns_state.id_to_index.get(&id) {
                ns_state.concept_vectors[*pos] = vector;
            }
            self.invalidate_cache(ns);
        } else {
            let pos = ns_state.concept_vectors.len();
            ns_state.concept_vectors.push(vector);
            ns_state.concept_indices.push(id.clone());
            ns_state.id_to_index.insert(id, pos);
        }

        Ok(())
    }

    pub fn update(&mut self, ns: &str, id: &str, vector: H) -> Result<()> {
        let ns_state = self.ensure_namespace(ns)?;
        if let Some(concept) = ns_state.concepts.get_mut(id) {
            concept.vector = vector;
            concept.modified_at = unix_now_secs();
            if let Some(pos) = ns_state.id_to_index.get(id) {
                ns_state.concept_vectors[*pos] = vector;
            }
            ns_state.index.insert(id.to_string(), &vector)?;
            self.invalidate_cache(ns);
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }

    pub fn delete(&mut self, ns: &str, id: &str) -> Result<()> {
        let ns_state = self.ensure_namespace(ns)?;
        if ns_state.concepts.remove(id).is_some() {
            ns_state.associations.remove(id);
            for neighbors in ns_state.associations.values_mut() {
                neighbors.remove(id);
            }
            if let Some(pos) = ns_state.id_to_index.remove(id) {
                let _ = ns_state.concept_vectors.swap_remove(pos);
                ns_state.concept_indices.swap_remove(pos);
                if pos < ns_state.concept_indices.len() {
                    let moved_id = &ns_state.concept_indices[pos];
                    ns_state.id_to_index.insert(moved_id.clone(), pos);
                }
            }
            ns_state.index.delete(id)?;
            self.invalidate_cache(ns);
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }

    pub fn clear(&mut self, ns: &str) {
        if let Some(ns_state) = self.namespaces.get_mut(ns) {
            ns_state.concepts.clear();
            ns_state.associations.clear();
            ns_state.concept_vectors.clear();
            ns_state.concept_indices.clear();
            ns_state.id_to_index.clear();
            let _ = ns_state.index.rebuild(&HashMap::new());
            self.invalidate_cache(ns);
        }
    }

    pub fn get(&self, ns: &str, id: &str) -> Option<&Concept<H>> {
        self.get_namespace(ns).and_then(|n| n.concepts.get(id))
    }

    pub fn associate(&mut self, ns: &str, from: &str, to: &str, strength: f32) -> Result<()> {
        // Validate strength before any other checks
        if !strength.is_finite() {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: "association strength must be finite".to_string(),
            });
        }
        if !(0.0..=1.0).contains(&strength) {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: format!("association strength must be in [0.0, 1.0], got {strength}"),
            });
        }

        // Read config limit before borrowing ns_state mutably
        let max_assoc = self.config.max_associations_per_concept;

        let ns_state = self.ensure_namespace(ns)?;
        if !ns_state.concepts.contains_key(from) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            });
        }
        if !ns_state.concepts.contains_key(to) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: to.to_string(),
            });
        }

        let neighbors = ns_state.associations.entry(from.to_string()).or_default();
        neighbors.insert(to.to_string(), (strength, unix_now_secs()));

        // Enforce max_associations_per_concept: evict weakest if over limit
        if let Some(limit) = max_assoc {
            while neighbors.len() > limit {
                if let Some(weakest) = neighbors
                    .iter()
                    .min_by(|a, b| {
                        a.1.0
                            .partial_cmp(&b.1.0)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(k, _)| k.clone())
                {
                    neighbors.remove(&weakest);
                } else {
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn disassociate(&mut self, ns: &str, from: &str, to: &str) -> Result<()> {
        let ns_state = self.ensure_namespace(ns)?;
        if let Some(neighbors) = ns_state.associations.get_mut(from) {
            neighbors.remove(to);
        }
        Ok(())
    }

    pub fn get_associations(&self, ns: &str, id: &str) -> Vec<(String, f32)> {
        self.get_associations_with_decay(ns, id, DecayCurve::None)
    }

    /// Get associations with decay curve applied.
    pub fn get_associations_with_decay(
        &self,
        ns: &str,
        id: &str,
        curve: DecayCurve,
    ) -> Vec<(String, f32)> {
        let now = unix_now_secs();
        self.get_namespace(ns)
            .and_then(|n| n.associations.get(id))
            .map(|m| {
                m.iter()
                    .map(|(k, (strength, created_at))| {
                        let elapsed = now.saturating_sub(*created_at);
                        (k.clone(), curve.apply(*strength, elapsed))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub fn incoming_associations(&self, ns: &str, id: &str) -> Vec<(String, f32)> {
        self.incoming_associations_with_decay(ns, id, DecayCurve::None)
    }

    /// Get incoming associations with decay curve applied.
    pub fn incoming_associations_with_decay(
        &self,
        ns: &str,
        id: &str,
        curve: DecayCurve,
    ) -> Vec<(String, f32)> {
        let now = unix_now_secs();
        let mut incoming = Vec::new();
        if let Some(ns_state) = self.get_namespace(ns) {
            for (from_id, neighbors) in &ns_state.associations {
                if let Some((strength, created_at)) = neighbors.get(id) {
                    let elapsed = now.saturating_sub(*created_at);
                    incoming.push((from_id.clone(), curve.apply(*strength, elapsed)));
                }
            }
        }
        incoming
            .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        incoming
    }

    pub fn all_concepts(&self, ns: &str) -> Vec<Concept<H>> {
        self.get_namespace(ns)
            .map(|n| n.concepts.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn all_associations(&self, ns: &str) -> Vec<(String, String, f32)> {
        self.all_associations_with_decay(ns, DecayCurve::None)
    }

    /// Get all associations with decay curve applied.
    pub fn all_associations_with_decay(
        &self,
        ns: &str,
        curve: DecayCurve,
    ) -> Vec<(String, String, f32)> {
        let now = unix_now_secs();
        let mut all = Vec::new();
        if let Some(ns_state) = self.get_namespace(ns) {
            for (from, neighbors) in &ns_state.associations {
                for (to, (strength, created_at)) in neighbors {
                    let elapsed = now.saturating_sub(*created_at);
                    all.push((from.clone(), to.clone(), curve.apply(*strength, elapsed)));
                }
            }
        }
        all
    }

    pub fn len(&self, ns: &str) -> usize {
        self.get_namespace(ns).map_or(0, |n| n.concepts.len())
    }

    pub fn is_empty(&self, ns: &str) -> bool {
        self.get_namespace(ns).is_none_or(|n| n.concepts.is_empty())
    }

    pub fn cache_metrics_snapshot(&self, ns: &str) -> CacheMetricsSnapshot {
        self.get_namespace(ns)
            .map_or(CacheMetricsSnapshot::default(), |n| {
                n.cache_metrics.snapshot()
            })
    }

    fn evict_oldest_if_needed(&mut self, ns: &str) {
        let Some(limit) = self.config.max_concepts else {
            return;
        };

        while self.len(ns) >= limit {
            let oldest = {
                let Some(ns_state) = self.get_namespace(ns) else {
                    break;
                };
                ns_state
                    .concepts
                    .values()
                    .min_by_key(|c| c.created_at)
                    .map(|c| c.id.clone())
            };

            if let Some(id) = oldest {
                let _ = self.delete(ns, &id);
            } else {
                break;
            }
        }
    }

    pub fn invalidate_cache(&self, ns: &str) {
        if let Some(ns_state) = self.get_namespace(ns) {
            if let Ok(mut cache) = ns_state.query_cache.write() {
                cache.clear();
            }
        }
    }

    pub fn index_stats(&self, ns: &str) -> IndexStats {
        self.get_namespace(ns)
            .map(|n| n.index.stats())
            .unwrap_or_default()
    }

    pub const fn retrieval_config(&self) -> &RetrievalConfig {
        &self._retrieval_config
    }
}

/// Get current time in Unix nanoseconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn unix_now_ns() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}

/// Get current time in Unix nanoseconds (WASM version).
#[cfg(target_arch = "wasm32")]
pub fn unix_now_ns() -> u64 {
    (js_sys::Date::new_0().get_time() * 1_000_000.0) as u64
}

pub fn similarity_cache_key<H: Hypervector>(query: &H, top_k: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut s = std::collections::hash_map::DefaultHasher::new();
    // Optimization: Hash the hypervector directly instead of calling to_bytes().
    // This eliminates a 1280-byte allocation/copy per cache lookup.
    query.hash(&mut s);
    top_k.hash(&mut s);
    s.finish()
}
