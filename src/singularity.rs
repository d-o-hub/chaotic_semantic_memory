//! Core concept storage and indexing engine

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::index::{AnnIndex, IndexBackend, IndexStats};
use crate::singularity_cache::CacheMetricsSnapshot;
use crate::singularity_retrieval::RetrievalConfig;
use crate::singularity_state::NamespaceState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;

/// Configuration for the Singularity engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityConfig {
    pub max_concepts: Option<usize>,
    pub max_associations_per_concept: Option<usize>,
    pub concept_cache_size: usize,
    pub max_cached_top_k: usize,
    pub index_backend: IndexBackend,
}

impl Default for SingularityConfig {
    fn default() -> Self {
        Self {
            max_concepts: None,
            max_associations_per_concept: None,
            concept_cache_size: 1000,
            max_cached_top_k: 100,
            index_backend: IndexBackend::BruteForce,
        }
    }
}

/// Represents a single memory concept
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Concept {
    pub id: String,
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    pub expires_at: Option<u64>,
    /// IDs of concepts that are "canonical" versions of this one (ADR-0044)
    #[serde(default)]
    pub canonical_concept_ids: Vec<String>,
}

/// Public summary of a historical version of a concept.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptVersion {
    pub version: u64,
    pub timestamp_unix: i64,
    pub vector_changed: bool,
    pub metadata_changed: bool,
}

/// Description of differences between two versions of a concept.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptDiff {
    pub vector_cosine_distance: f32,
    pub metadata_added: HashMap<String, serde_json::Value>,
    pub metadata_removed: HashMap<String, serde_json::Value>,
    pub metadata_changed: HashMap<String, (serde_json::Value, serde_json::Value)>,
}

impl ConceptDiff {
    /// Calculate the differences between two versions of a concept.
    pub fn calculate(from_concept: &Concept, to_concept: &Concept) -> Self {
        let sim = from_concept.vector.cosine_similarity(&to_concept.vector);
        let vector_cosine_distance = 1.0 - sim;

        let mut metadata_added = HashMap::new();
        let mut metadata_removed = HashMap::new();
        let mut metadata_changed = HashMap::new();

        // Find added and changed
        for (k, v_to) in &to_concept.metadata {
            if let Some(v_from) = from_concept.metadata.get(k) {
                if v_from != v_to {
                    metadata_changed.insert(k.clone(), (v_from.clone(), v_to.clone()));
                }
            } else {
                metadata_added.insert(k.clone(), v_to.clone());
            }
        }

        // Find removed
        for (k, v_from) in &from_concept.metadata {
            if !to_concept.metadata.contains_key(k) {
                metadata_removed.insert(k.clone(), v_from.clone());
            }
        }

        Self {
            vector_cosine_distance,
            metadata_added,
            metadata_removed,
            metadata_changed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConceptBuilder {
    id: String,
    vector: Option<HVec10240>,
    metadata: HashMap<String, serde_json::Value>,
    expires_at: Option<u64>,
    canonical_concept_ids: Vec<String>,
}

impl ConceptBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            vector: None,
            metadata: HashMap::new(),
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }

    pub const fn with_vector(mut self, vector: HVec10240) -> Self {
        self.vector = Some(vector);
        self
    }

    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.expires_at = Some(unix_now_secs() + ttl_secs);
        self
    }

    pub fn build(self) -> Result<Concept> {
        let now = unix_now_secs();
        Ok(Concept {
            id: self.id,
            vector: self.vector.unwrap_or_else(HVec10240::random),
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
            expires_at: self.expires_at,
            canonical_concept_ids: self.canonical_concept_ids,
        })
    }
}

pub struct Singularity {
    pub(crate) config: SingularityConfig,
    pub(crate) namespaces: HashMap<String, NamespaceState>,
    pub(crate) _retrieval_config: RetrievalConfig,
}

impl Singularity {
    pub fn new(config: SingularityConfig) -> Self {
        Self {
            config,
            namespaces: HashMap::new(),
            _retrieval_config: RetrievalConfig::default(),
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

    fn create_index(&self) -> Box<dyn AnnIndex> {
        crate::index::create_index(&self.config.index_backend)
            .expect("ANN index creation failed; check feature flags and configuration")
    }

    pub(crate) fn get_namespace(&self, ns: &str) -> Option<&NamespaceState> {
        self.namespaces.get(ns)
    }

    pub(crate) fn get_namespace_mut(&mut self, ns: &str) -> &mut NamespaceState {
        if !self.namespaces.contains_key(ns) {
            let index = self.create_index();
            self.namespaces
                .insert(ns.to_string(), NamespaceState::new(&self.config, index));
        }
        self.namespaces.get_mut(ns).unwrap()
    }

    #[instrument(skip(self, concept))]
    pub fn inject(&mut self, ns: &str, concept: Concept) -> Result<()> {
        self.evict_oldest_if_needed(ns);
        let id = concept.id.clone();
        let vector = concept.vector;

        let ns_state = self.get_namespace_mut(ns);

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

    pub fn update(&mut self, ns: &str, id: &str, vector: HVec10240) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
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
        let ns_state = self.get_namespace_mut(ns);
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

    pub fn get(&self, ns: &str, id: &str) -> Option<&Concept> {
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

        let ns_state = self.get_namespace_mut(ns);
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
        neighbors.insert(to.to_string(), strength);

        // Enforce max_associations_per_concept: evict weakest if over limit
        if let Some(limit) = max_assoc {
            while neighbors.len() > limit {
                if let Some(weakest) = neighbors
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
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
        let ns_state = self.get_namespace_mut(ns);
        if let Some(neighbors) = ns_state.associations.get_mut(from) {
            neighbors.remove(to);
        }
        Ok(())
    }

    pub fn get_associations(&self, ns: &str, id: &str) -> Vec<(String, f32)> {
        self.get_namespace(ns)
            .and_then(|n| n.associations.get(id))
            .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn incoming_associations(&self, ns: &str, id: &str) -> Vec<(String, f32)> {
        let mut incoming = Vec::new();
        if let Some(ns_state) = self.get_namespace(ns) {
            for (from_id, neighbors) in &ns_state.associations {
                if let Some(strength) = neighbors.get(id) {
                    incoming.push((from_id.clone(), *strength));
                }
            }
        }
        incoming.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        incoming
    }

    pub fn all_concepts(&self, ns: &str) -> Vec<Concept> {
        self.get_namespace(ns)
            .map(|n| n.concepts.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn all_associations(&self, ns: &str) -> Vec<(String, String, f32)> {
        let mut all = Vec::new();
        if let Some(ns_state) = self.get_namespace(ns) {
            for (from, neighbors) in &ns_state.associations {
                for (to, strength) in neighbors {
                    all.push((from.clone(), to.clone(), *strength));
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

pub fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn unix_now_ns() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}

pub(crate) fn similarity_cache_key(query: &HVec10240, top_k: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut s = std::collections::hash_map::DefaultHasher::new();
    query.data.hash(&mut s);
    top_k.hash(&mut s);
    s.finish()
}
