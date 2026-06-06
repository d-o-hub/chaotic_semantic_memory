//! Concept builder for ergonomic concept construction

use serde::Serialize;
use std::collections::HashMap;

use csm_core::error::{MemoryError, Result};
use csm_core::hyperdim::HVec10240;
use crate::singularity::Concept;

/// Builder for constructing [`Concept`] instances with a fluent API.
///
/// # Example
///
/// ```
/// use chaotic_semantic_memory::singularity::ConceptBuilder;
/// use chaotic_semantic_memory::HVec10240;
///
/// let concept = ConceptBuilder::new("example")
///     .with_vector(HVec10240::random())
///     .with_metadata("source", "test")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct ConceptBuilder {
    id: String,
    vector: Option<HVec10240>,
    metadata: HashMap<String, serde_json::Value>,
    metadata_error: Option<MemoryError>,
    ttl_seconds: Option<u64>,
    canonical_concept_ids: Vec<String>,
}

impl ConceptBuilder {
    /// Creates a new builder for a concept with the given ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            vector: None,
            metadata: HashMap::new(),
            metadata_error: None,
            ttl_seconds: None,
            canonical_concept_ids: Vec::new(),
        }
    }

    /// Sets the canonical concept IDs for semantic bridge linking.
    #[must_use]
    pub fn with_canonical_concepts(mut self, ids: Vec<String>) -> Self {
        self.canonical_concept_ids = ids;
        self
    }

    /// Sets the TTL (time to live) in seconds for this concept.
    ///
    /// The concept will expire after `ttl_seconds` from creation.
    /// If not set, the concept never expires.
    #[must_use]
    pub const fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self
    }

    /// Sets the vector for this concept.
    #[must_use]
    pub const fn with_vector(mut self, vector: HVec10240) -> Self {
        self.vector = Some(vector);
        self
    }

    /// Adds metadata to this concept.
    ///
    /// If serialization of the value fails, the error is captured and
    /// will be returned when `build()` is called.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if self.metadata_error.is_none() {
            match serde_json::to_value(value) {
                Ok(value) => {
                    self.metadata.insert(key.into(), value);
                }
                Err(error) => {
                    self.metadata_error = Some(MemoryError::Serialization(error));
                }
            }
        }
        self
    }

    /// Builds the [`Concept`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata serialization failed during construction.
    pub fn build(self) -> Result<Concept> {
        if let Some(error) = self.metadata_error {
            return Err(error);
        }

        let now = crate::singularity::unix_now_secs();
        let expires_at = self.ttl_seconds.map(|ttl| now + ttl);

        Ok(Concept {
            id: self.id,
            vector: self.vector.unwrap_or_else(HVec10240::random),
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
            expires_at,
            canonical_concept_ids: self.canonical_concept_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concept_builder_creates_concept_with_metadata() {
        let concept = ConceptBuilder::new("test-id")
            .with_vector(HVec10240::random())
            .with_metadata("key1", "value1")
            .with_metadata("key2", 42i32)
            .build()
            .unwrap();

        assert_eq!(concept.id, "test-id");
        assert_eq!(
            concept.metadata.get("key1").unwrap().as_str().unwrap(),
            "value1"
        );
        assert_eq!(concept.metadata.get("key2").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn concept_builder_uses_random_vector_by_default() {
        let concept = ConceptBuilder::new("test").build().unwrap();
        // Just verify it builds successfully without explicit vector
        assert_eq!(concept.id, "test");
    }

    #[test]
    fn concept_builder_with_ttl_sets_expiration() {
        let now = crate::singularity::unix_now_secs();
        let concept = ConceptBuilder::new("ttl-test")
            .with_ttl(3600)
            .build()
            .unwrap();

        assert!(concept.expires_at.is_some());
        let expires_at = concept.expires_at.unwrap();
        // Expiration should be approximately now + 3600
        assert!(expires_at >= now + 3600 - 1);
        assert!(expires_at <= now + 3600 + 1);
    }

    #[test]
    fn concept_builder_without_ttl_has_no_expiration() {
        let concept = ConceptBuilder::new("no-ttl").build().unwrap();
        assert!(concept.expires_at.is_none());
    }
}
