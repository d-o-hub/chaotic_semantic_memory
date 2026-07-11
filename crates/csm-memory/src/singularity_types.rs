//! Type definitions for the Singularity engine.
//!
//! Contains core data types: `Concept`, `Association`, `DecayCurve`,
//! `ConceptVersion`, `ConceptDiff`, `ConceptBuilder`, and `SingularityConfig`.

use crate::index::IndexBackend;
use csm_core::error::Result;
use csm_core::hyperdim::{HVec10240, Hypervector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[serde(bound = "H: Hypervector")]
pub struct Concept<H: Hypervector = HVec10240> {
    pub id: String,
    pub vector: H,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    pub expires_at: Option<u64>,
    /// IDs of concepts that are "canonical" versions of this one (ADR-0044)
    #[serde(default)]
    pub canonical_concept_ids: Vec<String>,
}

/// Represents an association between two concepts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Association {
    pub strength: f32,
    pub created_at: u64,
}

/// Curve defining how association strength decays over time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum DecayCurve {
    /// No decay (static strength).
    #[default]
    None,
    /// Linear decay: strength = strength * (1 - elapsed / limit).
    Linear {
        /// Time in seconds until strength reaches zero.
        limit_seconds: u64,
    },
    /// Exponential decay: strength = strength * e^(-t / tau).
    Exponential {
        /// Time in seconds for strength to halve.
        half_life_seconds: u64,
    },
    /// Step decay: strength drops by a fixed amount after a threshold.
    Step {
        /// Time in seconds after which the drop occurs.
        threshold_seconds: u64,
        /// Amount to subtract from strength (clamped to 0.0).
        drop: f32,
    },
}

impl DecayCurve {
    /// Apply decay curve to a strength given elapsed time.
    #[allow(clippy::cast_precision_loss)]
    pub fn apply(&self, strength: f32, elapsed_secs: u64) -> f32 {
        match self {
            Self::None => strength,
            Self::Linear { limit_seconds } => {
                if elapsed_secs >= *limit_seconds {
                    0.0
                } else {
                    strength * (1.0 - (elapsed_secs as f32 / *limit_seconds as f32))
                }
            }
            Self::Exponential { half_life_seconds } => {
                let lambda = std::f32::consts::LN_2 / (*half_life_seconds as f32);
                strength * (-lambda * elapsed_secs as f32).exp()
            }
            Self::Step {
                threshold_seconds,
                drop,
            } => {
                if elapsed_secs >= *threshold_seconds {
                    (strength - drop).max(0.0)
                } else {
                    strength
                }
            }
        }
    }
}

/// Represents a historical version of a concept.
/// Can be a summary (with change flags) or a full record (with vector/metadata).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(bound = "H: Hypervector")]
pub struct ConceptVersion<H: Hypervector = HVec10240> {
    pub concept_id: String,
    pub version: u64,
    pub timestamp_unix: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<H>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_changed: Option<bool>,
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
    pub fn calculate<H: Hypervector>(from_concept: &Concept<H>, to_concept: &Concept<H>) -> Self {
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
#[allow(dead_code)]
pub struct ConceptBuilder<H: Hypervector = HVec10240> {
    id: String,
    vector: Option<H>,
    metadata: HashMap<String, serde_json::Value>,
    expires_at: Option<u64>,
    canonical_concept_ids: Vec<String>,
}

#[allow(dead_code)]
impl<H: Hypervector> ConceptBuilder<H> {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            vector: None,
            metadata: HashMap::new(),
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn with_vector(mut self, vector: H) -> Self {
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

    pub fn build(self) -> Result<Concept<H>> {
        let now = unix_now_secs();
        Ok(Concept {
            id: self.id,
            vector: self.vector.unwrap_or_else(H::random),
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
            expires_at: self.expires_at,
            canonical_concept_ids: self.canonical_concept_ids,
        })
    }
}

/// Get current time in Unix seconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Get current time in Unix seconds (WASM version).
#[cfg(target_arch = "wasm32")]
pub fn unix_now_secs() -> u64 {
    (js_sys::Date::new_0().get_time() / 1000.0) as u64
}
