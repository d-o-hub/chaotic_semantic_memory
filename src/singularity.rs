//! Episode-free concept injection

// Casts are intentional for similarity math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

#[cfg(target_arch = "wasm32")]
use js_sys::Date;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::index::AnnIndex;
use crate::index::IndexBackend;
use crate::index::brute_force::BruteForce;
pub use crate::singularity_cache::CacheMetricsSnapshot;
pub use crate::singularity_retrieval::{CandidateSource, RetrievalConfig, RetrievalStats};
use crate::singularity_state::NamespaceState;

const DEFAULT_CONCEPT_CACHE_SIZE: usize = 128;
pub const DEFAULT_MAX_CACHED_TOP_K: usize = 100;

/// A concept in semantic memory.
///
/// Use [`ConceptBuilder`] to construct instances with proper defaults.
/// Direct struct construction is supported but the `expires_at` field should
/// default to `None` for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    #[serde(default)]
    pub expires_at: Option<u64>,
    /// Links to canonical concepts for semantic bridge scoring.
    #[serde(default)]
    pub canonical_concept_ids: Vec<String>,
}

#[derive(Debug, Clone)]
/// Runtime configuration for [`Singularity`].
pub struct SingularityConfig {
    /// Maximum concept count before eviction (default: `None`).
    pub max_concepts: Option<usize>,
    /// Maximum outbound associations per concept (default: `None`).
    pub max_associations_per_concept: Option<usize>,
    /// LRU cache capacity for similarity results (default: `128`, coerced to `>= 1`).
    pub concept_cache_size: usize,
    /// Maximum top_k for cache eligibility (default: `100`).
    pub max_cached_top_k: usize,
}
impl Default for SingularityConfig {
    fn default() -> Self {
        Self {
            max_concepts: None,
            max_associations_per_concept: None,
            concept_cache_size: DEFAULT_CONCEPT_CACHE_SIZE,
            max_cached_top_k: DEFAULT_MAX_CACHED_TOP_K,
        }
    }
}

/// Episode-free singularity engine
#[derive(Debug)]
pub struct Singularity {
    pub(crate) namespaces: HashMap<String, NamespaceState>,
    pub(crate) config: SingularityConfig,
    pub(crate) retrieval_config: RetrievalConfig,
    pub(crate) index_backend: IndexBackend,
}
impl Singularity {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(SingularityConfig::default())
    }

    pub fn with_config(config: SingularityConfig) -> Self {
        Self::with_config_and_backend(config, IndexBackend::BruteForce)
    }

    pub fn with_config_and_backend(config: SingularityConfig, backend: IndexBackend) -> Self {
        Self {
            namespaces: HashMap::new(),
            config,
            retrieval_config: RetrievalConfig::default(),
            index_backend: backend,
        }
    }

    pub(crate) fn create_index(&self) -> Box<dyn AnnIndex> {
        match &self.index_backend {
            IndexBackend::BruteForce => Box::new(BruteForce::new()),
            #[cfg(feature = "ann-hnsw")]
            IndexBackend::Hnsw {
                m,
                ef_construction,
                ef_search,
            } => Box::new(crate::index::hnsw::HnswIndex::new(
                *m,
                *ef_construction,
                *ef_search,
            )),
            #[cfg(not(feature = "ann-hnsw"))]
            IndexBackend::Hnsw { .. } => Box::new(BruteForce::new()),
            #[cfg(feature = "ann-lsh")]
            IndexBackend::Lsh {
                num_tables,
                hash_bits,
            } => Box::new(crate::index::lsh::LshIndex::new(*num_tables, *hash_bits)),
            #[cfg(not(feature = "ann-lsh"))]
            IndexBackend::Lsh { .. } => Box::new(BruteForce::new()),
        }
    }

    pub(crate) fn get_namespace_mut(&mut self, ns: &str) -> &mut NamespaceState {
        if !self.namespaces.contains_key(ns) {
            let index = self.create_index();
            self.namespaces.insert(ns.to_string(), NamespaceState::new(&self.config, index));
        }
        self.namespaces.get_mut(ns).unwrap()
    }

    pub(crate) fn get_namespace(&self, ns: &str) -> Option<&NamespaceState> {
        self.namespaces.get(ns)
    }
    /// Inject a concept directly into memory
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, concept), fields(concept_id = %concept.id)))]
    pub fn inject(&mut self, ns: &str, concept: Concept) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
        let is_new = !ns_state.concepts.contains_key(&concept.id);
        if is_new {
            self.evict_oldest_if_needed(ns);
        }

        let ns_state = self.get_namespace_mut(ns);
        if let Some(&idx) = ns_state.id_to_index.get(&concept.id) {
            ns_state.concept_vectors[idx] = concept.vector;
        } else {
            let idx = ns_state.concept_indices.len();
            ns_state.id_to_index.insert(concept.id.clone(), idx);
            ns_state.concept_indices.push(concept.id.clone());
            ns_state.concept_vectors.push(concept.vector);
        }

        let concept_id = concept.id.clone();
        let concept_vector = concept.vector;
        ns_state.concepts.insert(concept_id.clone(), concept);
        ns_state.index.insert(concept_id, &concept_vector)?;
        self.invalidate_cache(ns);
        Ok(())
    }

    /// Retrieve concept by ID
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(concept_id = %id)))]
    pub fn get(&self, ns: &str, id: &str) -> Option<&Concept> {
        self.get_namespace(ns).and_then(|n| n.concepts.get(id))
    }

    /// Delete concept by ID
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(concept_id = %id)))]
    pub fn delete(&mut self, ns: &str, id: &str) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
        if let Some(idx) = ns_state.id_to_index.remove(id) {
            ns_state.concept_indices.swap_remove(idx);
            let _ = ns_state.concept_vectors.swap_remove(idx);
            if idx < ns_state.concept_indices.len() {
                let swapped_id = &ns_state.concept_indices[idx];
                ns_state.id_to_index.insert(swapped_id.clone(), idx);
            }
        }

        ns_state.concepts.remove(id);
        let _ = ns_state.index.delete(id);
        ns_state.associations.remove(id);
        for links in ns_state.associations.values_mut() {
            links.remove(id);
        }
        self.invalidate_cache(ns);
        Ok(())
    }

    /// Clear all concepts and associations
    pub fn clear(&mut self, ns: &str) {
        let ns_state = self.get_namespace_mut(ns);
        ns_state.concepts.clear();
        ns_state.associations.clear();
        ns_state.concept_indices.clear();
        ns_state.concept_vectors.clear();
        ns_state.id_to_index.clear();
        let _ = ns_state.index.rebuild(&ns_state.concepts);
        self.invalidate_cache(ns);
    }

    /// Update concept vector
    pub fn update(&mut self, ns: &str, id: &str, new_vector: HVec10240) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
        if let Some(&idx) = ns_state.id_to_index.get(id) {
            ns_state.concept_vectors[idx] = new_vector;
        }

        if let Some(concept) = ns_state.concepts.get_mut(id) {
            concept.vector = new_vector;
            concept.modified_at = unix_now_secs();
            ns_state.index.insert(id.to_string(), &new_vector)?;
            self.invalidate_cache(ns);
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }

    /// Create or update association between concepts
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(from_id = %from, to_id = %to, strength = strength)))]
    pub fn associate(&mut self, ns: &str, from: &str, to: &str, strength: f32) -> Result<()> {
        let max_assoc = self.config.max_associations_per_concept;
        let ns_state = self.get_namespace_mut(ns);
        if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
            return Err(MemoryError::InvalidInput {
                field: "strength".to_string(),
                reason: "must be finite and between 0.0 and 1.0".to_string(),
            });
        }
        if !ns_state.concepts.contains_key(from) || !ns_state.concepts.contains_key(to) {
            let missing = if !ns_state.concepts.contains_key(from) {
                from
            } else {
                to
            };
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: missing.to_string(),
            });
        }

        let links = ns_state.associations.entry(from.to_string()).or_default();
        links.insert(to.to_string(), strength);

        if let Some(limit) = max_assoc {
            while links.len() > limit {
                if let Some((weakest, _)) = links
                    .iter()
                    .min_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(k, v)| (k.clone(), *v))
                {
                    links.remove(&weakest);
                } else {
                    break;
                }
            }
        }

        self.invalidate_cache(ns);
        Ok(())
    }

    /// Get associations for a concept
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(concept_id = %id)))]
    pub fn get_associations(&self, ns: &str, id: &str) -> Vec<(String, f32)> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Vec::new();
        };
        let mut results: Vec<(String, f32)> = ns_state
            .associations
            .get(id)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default();
        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        results
    }

    /// Bundle multiple concepts into a single hypervector
    pub fn bundle_concepts(&self, ns: &str, ids: &[String]) -> Result<HVec10240> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Err(MemoryError::NotFound {
                entity: "Namespace".to_string(),
                id: ns.to_string(),
            });
        };
        let vectors: Vec<_> = ids
            .iter()
            .filter_map(|id| ns_state.concepts.get(id))
            .map(|c| c.vector)
            .collect();

        HVec10240::bundle(&vectors)
    }

    pub fn concept_ids(&self, ns: &str) -> Vec<String> {
        self.get_namespace(ns)
            .map(|n| n.concepts.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn all_concepts(&self, ns: &str) -> Vec<Concept> {
        self.get_namespace(ns)
            .map(|n| n.concepts.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn all_associations(&self, ns: &str) -> Vec<(String, String, f32)> {
        let mut output = Vec::new();
        if let Some(ns_state) = self.get_namespace(ns) {
            for (from, links) in &ns_state.associations {
                for (to, strength) in links {
                    output.push((from.clone(), to.clone(), *strength));
                }
            }
        }
        output
    }

    pub fn len(&self, ns: &str) -> usize {
        self.get_namespace(ns).map_or(0, |n| n.concepts.len())
    }

    pub fn is_empty(&self, ns: &str) -> bool {
        self.get_namespace(ns).map_or(true, |n| n.concepts.is_empty())
    }

    pub fn cache_metrics_snapshot(&self, ns: &str) -> CacheMetricsSnapshot {
        self.get_namespace(ns)
            .map_or(CacheMetricsSnapshot::default(), |n| n.cache_metrics.snapshot())
    }

    fn evict_oldest_if_needed(&mut self, ns: &str) {
        let Some(limit) = self.config.max_concepts else {
            return;
        };

        while self.len(ns) >= limit {
            let oldest = {
                let ns_state = self.get_namespace(ns).unwrap();
                ns_state
                    .concepts
                    .values()
                    .min_by_key(|c| c.created_at)
                    .map(|c| c.id.clone())
            };

            if let Some(oldest_id) = oldest {
                self.delete(ns, &oldest_id).ok();
                self.invalidate_cache(ns);
            } else {
                break;
            }
        }
    }

    pub(crate) fn invalidate_cache(&self, ns: &str) {
        if let Some(ns_state) = self.get_namespace(ns) {
            if let Ok(mut cache) = ns_state.query_cache.write() {
                cache.clear();
            }
        }
    }
}

impl Default for Singularity {
    fn default() -> Self {
        Self::with_config(SingularityConfig::default())
    }
}

/// Get current Unix timestamp in seconds
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn unix_now_secs() -> u64 {
    let millis = Date::now();
    if !millis.is_finite() || millis < 0.0 {
        return 0;
    }
    let secs = (millis / 1000.0).floor();
    format!("{secs:.0}").parse::<u64>().unwrap_or(0)
}

/// Get current Unix timestamp in nanoseconds
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn unix_now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn unix_now_ns() -> u64 {
    let millis = Date::now();
    if !millis.is_finite() || millis < 0.0 {
        return 0;
    }
    let nanos = (millis * 1_000_000.0).floor();
    format!("{nanos:.0}").parse::<u64>().unwrap_or(0)
}

/// Generate cache key for similarity query
pub(crate) fn similarity_cache_key(query: &HVec10240, top_k: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    top_k.hash(&mut hasher);
    query.data.hash(&mut hasher);
    hasher.finish()
}

// Re-export ConceptBuilder from the dedicated module
pub use crate::concept_builder::ConceptBuilder;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;
    fn c(id: &str) -> Concept {
        ConceptBuilder::new(id)
            .with_vector(HVec10240::random())
            .build()
            .unwrap()
    }
    #[test]
    fn crud() {
        let mut s = Singularity::new();
        let ns = "_default";
        s.inject(ns, c("x")).unwrap();
        assert!(s.get(ns, "x").is_some());
        s.delete(ns, "x").unwrap();
        assert!(s.get(ns, "x").is_none() && s.get_namespace(ns).unwrap().id_to_index.is_empty());
        assert!(s.delete(ns, "m").is_ok());
    }
    #[test]
    fn update() {
        let mut s = Singularity::new();
        let ns = "_default";
        s.inject(ns, c("x")).unwrap();
        let v = HVec10240::random();
        s.update(ns, "x", v).unwrap();
        assert_eq!(s.get(ns, "x").unwrap().vector, v);
        assert!(s.update(ns, "m", HVec10240::random()).is_err());
    }
    #[test]
    fn assoc() {
        let mut s = Singularity::new();
        let ns = "_default";
        s.inject(ns, c("a")).unwrap();
        s.inject(ns, c("b")).unwrap();
        s.associate(ns, "a", "b", 0.5).unwrap();
        assert_eq!(s.get_associations(ns, "a"), vec![("b".into(), 0.5)]);
        assert!(s.associate(ns, "a", "m", 0.5).is_err());
        assert!(s.associate(ns, "a", "b", -1.0).is_err());
        assert!(s.associate(ns, "a", "b", f32::NAN).is_err());
    }
    #[test]
    fn similar_empty() {
        let empty = Singularity::new();
        let ns = "_default";
        assert!(empty.find_similar(ns, &HVec10240::random(), 5).is_empty());
        let mut s = Singularity::new();
        s.inject(ns, c("x")).unwrap();
        assert!(s.find_similar(ns, &HVec10240::random(), 0).is_empty());
    }
    #[test]
    fn clear_all() {
        let mut s = Singularity::new();
        let ns = "_default";
        s.inject(ns, c("x")).unwrap();
        s.associate(ns, "x", "x", 0.5).unwrap();
        s.clear(ns);
        let ns_state = s.get_namespace(ns).unwrap();
        assert!(s.is_empty(ns) && ns_state.associations.is_empty() && ns_state.concept_indices.is_empty());
    }
}
