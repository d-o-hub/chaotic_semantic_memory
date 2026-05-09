//! Concept builder for ergonomic concept construction

use crate::error::Result;
use crate::hyperdim::{HVec10240, Hypervector};
use crate::singularity::{Concept, unix_now_secs};
use std::collections::HashMap;

/// Builder for constructing [`Concept`] instances with a fluent API.
#[derive(Debug, Clone)]
pub struct ConceptBuilder<H: Hypervector = HVec10240> {
    id: String,
    vector: Option<H>,
    metadata: HashMap<String, serde_json::Value>,
    expires_at: Option<u64>,
    canonical_concept_ids: Vec<String>,
}

impl<H: Hypervector> ConceptBuilder<H> {
    /// Creates a new builder for the specified concept ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            vector: None,
            metadata: HashMap::new(),
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }

    /// Sets the concept's hypervector.
    pub const fn with_vector(mut self, vector: H) -> Self {
        self.vector = Some(vector);
        self
    }

    /// Adds a metadata field to the concept.
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Sets multiple metadata fields.
    pub fn with_full_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the time-to-live (TTL) for the concept in seconds.
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.expires_at = Some(unix_now_secs() + ttl_secs);
        self
    }

    /// Sets the canonical concept IDs that this concept is associated with.
    pub fn with_canonical_ids(mut self, ids: Vec<String>) -> Self {
        self.canonical_concept_ids = ids;
        self
    }

    /// Builds the [`Concept`] instance.
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
