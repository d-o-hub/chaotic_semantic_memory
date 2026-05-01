//! TTL (Time-To-Live) and text convenience operations for ChaoticSemanticFramework.

use crate::error::Result;
use crate::framework_events::MemoryEvent;
use crate::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
use crate::singularity::ConceptBuilder;
use std::collections::HashMap;
use tracing::instrument;

impl crate::framework::ChaoticSemanticFramework {
    /// Inject a concept with TTL (time to live) into memory.
    ///
    /// The concept will expire after `ttl_seconds` from creation.
    /// Expired concepts are automatically filtered during probe operations.
    #[instrument(err, skip(self, id, vector))]
    pub async fn inject_concept_with_ttl(
        &self,
        id: impl Into<String>,
        vector: HVec10240,
        ttl_seconds: u64,
    ) -> Result<()> {
        let id = id.into();
        Self::validate_concept_id(&id)?;
        let concept = ConceptBuilder::new(id.clone())
            .with_vector(vector)
            .with_ttl(ttl_seconds)
            .build()?;

        {
            let mut sing = self.singularity.write().await;
            sing.inject(concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concept(&concept).await?;
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id,
            timestamp: concept.modified_at,
        });

        Ok(())
    }

    /// Inject a concept from text with TTL.
    #[instrument(err, skip(self, text))]
    pub async fn inject_text_with_ttl(&self, id: &str, text: &str, ttl_seconds: u64) -> Result<()> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(text);
        self.inject_concept_with_ttl(id, vector, ttl_seconds).await
    }

    /// Purge all expired concepts from memory.
    ///
    /// Returns the number of concepts removed.
    #[instrument(err, skip(self))]
    pub async fn purge_expired(&self) -> Result<usize> {
        let count = {
            let mut sing = self.singularity.write().await;
            sing.purge_expired()
        };
        Ok(count)
    }

    /// Inject a concept from text using the built-in encoder.
    ///
    /// The text is encoded to a hypervector using `TextEncoder` and stored
    /// with the given ID. This is a convenience method for the common case
    /// of storing text-based concepts.
    pub async fn inject_text(&self, id: &str, text: &str) -> Result<()> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(text);
        self.inject_concept(id, vector).await
    }

    /// Inject a concept from text with metadata.
    pub async fn inject_text_with_metadata(
        &self,
        id: &str,
        text: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(text);
        self.inject_concept_with_metadata(id, vector, metadata)
            .await
    }

    /// Probe for similar concepts using text input.
    ///
    /// Encodes the query text and finds the most similar concepts.
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(query);
        self.probe(vector, top_k).await
    }

    /// Probe for similar concepts using text input and metadata filtering.
    pub async fn probe_text_filtered(
        &self,
        query: &str,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> Result<Vec<(String, f32)>> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(query);
        self.probe_filtered(&vector, top_k, filter).await
    }

    /// Query specifically for a session using text input.
    ///
    /// This is a convenience method that filters results to only those
    /// with a `session_id` metadata field matching the provided ID.
    pub async fn query_in_session(
        &self,
        query: &str,
        session_id: &str,
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        let filter = MetadataFilter::eq("session_id", session_id);
        self.probe_text_filtered(query, top_k, &filter).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::significant_drop_tightening)] // Locks held during test assertions

    use super::*;
    use crate::framework::ChaoticSemanticFramework;
    use crate::singularity::unix_now_secs;

    #[tokio::test]
    async fn inject_concept_with_ttl_sets_expires_at() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let before = unix_now_secs();
        let vector = HVec10240::random();
        framework
            .inject_concept_with_ttl("ttl-concept", vector, 3600)
            .await
            .unwrap();
        let after = unix_now_secs();

        // Verify concept was stored and has correct expires_at
        let sing = framework.singularity.read().await;
        let concept = sing.get("ttl-concept").expect("concept should exist");
        assert!(concept.expires_at.is_some(), "expires_at should be set");

        let expires_at = concept.expires_at.unwrap();
        // expires_at should be approximately now + 3600
        let expected_min = before + 3600;
        let expected_max = after + 3600;
        assert!(
            expires_at >= expected_min && expires_at <= expected_max,
            "expires_at should be between {expected_min} and {expected_max}, got {expires_at}"
        );
    }

    #[tokio::test]
    async fn inject_text_with_ttl_encodes_and_stores() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        framework
            .inject_text_with_ttl("text-ttl", "hello world", 1800)
            .await
            .unwrap();

        // Verify concept exists with TTL
        let sing = framework.singularity.read().await;
        let concept = sing.get("text-ttl").expect("concept should exist");
        assert!(concept.expires_at.is_some(), "expires_at should be set");
    }

    #[tokio::test]
    async fn inject_text_without_ttl_no_expiration() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        framework
            .inject_text("no-ttl-text", "persistent content")
            .await
            .unwrap();

        let sing = framework.singularity.read().await;
        let concept = sing.get("no-ttl-text").expect("concept should exist");
        assert!(
            concept.expires_at.is_none(),
            "concept without TTL should not have expires_at"
        );
    }

    #[tokio::test]
    async fn inject_text_with_metadata_no_ttl() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), serde_json::json!("test"));
        metadata.insert("priority".to_string(), serde_json::json!(5));

        framework
            .inject_text_with_metadata("meta-concept", "test content", metadata)
            .await
            .unwrap();

        let sing = framework.singularity.read().await;
        let concept = sing.get("meta-concept").expect("concept should exist");
        assert!(
            concept.expires_at.is_none(),
            "inject_text_with_metadata should not set TTL"
        );
        assert_eq!(
            concept.metadata.get("source").unwrap().as_str().unwrap(),
            "test"
        );
        assert_eq!(
            concept.metadata.get("priority").unwrap().as_i64().unwrap(),
            5
        );
    }

    #[tokio::test]
    async fn probe_text_returns_similar_concepts() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        framework
            .inject_text("doc1", "machine learning algorithms")
            .await
            .unwrap();
        framework
            .inject_text("doc2", "neural network deep learning")
            .await
            .unwrap();
        framework
            .inject_text("doc3", "cooking recipes food")
            .await
            .unwrap();

        let results = framework.probe_text("learning", 5).await.unwrap();
        assert!(!results.is_empty(), "should find similar concepts");

        // Learning-related docs should rank higher than cooking
        let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
        assert!(
            ids.contains(&"doc1") || ids.contains(&"doc2"),
            "should find learning-related docs"
        );
    }

    #[tokio::test]
    async fn purge_expired_removes_only_expired() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        // Inject concept with short TTL (1 second)
        let vector = HVec10240::random();
        framework
            .inject_concept_with_ttl("short-ttl", vector, 1)
            .await
            .unwrap();

        // Inject concept with long TTL
        framework
            .inject_concept_with_ttl("long-ttl", HVec10240::random(), 3600)
            .await
            .unwrap();

        // Inject concept without TTL
        framework
            .inject_text("no-ttl", "persistent content")
            .await
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

        // Purge expired
        let purged = framework.purge_expired().await.unwrap();
        assert!(purged >= 1, "At least one concept should be purged");

        // Verify remaining concepts
        let sing = framework.singularity.read().await;
        assert!(
            !sing.concepts.contains_key("short-ttl"),
            "expired concept should be purged"
        );
        assert!(
            sing.concepts.contains_key("long-ttl"),
            "long TTL concept should remain"
        );
        assert!(
            sing.concepts.contains_key("no-ttl"),
            "concept without TTL should remain"
        );
    }

    #[tokio::test]
    async fn concept_expires_at_serialization() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let now = unix_now_secs();
        let ttl = 7200;
        let vector = HVec10240::random();

        framework
            .inject_concept_with_ttl("serial-test", vector, ttl)
            .await
            .unwrap();

        let sing = framework.singularity.read().await;
        let concept = sing.get("serial-test").unwrap();

        // Verify expires_at was computed correctly
        let expected_min = now + ttl;
        let expected_max = now + ttl + 1;
        assert!(
            concept.expires_at.unwrap() >= expected_min
                && concept.expires_at.unwrap() <= expected_max,
            "expires_at should be now + ttl"
        );

        // Verify JSON serialization of expires_at
        let json = serde_json::to_string(concept).unwrap();
        assert!(
            json.contains("expires_at"),
            "JSON should contain expires_at field"
        );

        // Verify deserialization
        let deserialized: crate::singularity::Concept =
            serde_json::from_str(&json).expect("should deserialize concept");
        assert_eq!(
            deserialized.expires_at, concept.expires_at,
            "expires_at should round-trip"
        );
    }

    #[tokio::test]
    async fn multiple_ttl_concepts_independent_expiry() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        // Inject multiple concepts with different TTLs (using larger margins for test stability)
        framework
            .inject_concept_with_ttl("first", HVec10240::random(), 1)
            .await
            .unwrap();
        framework
            .inject_concept_with_ttl("second", HVec10240::random(), 3)
            .await
            .unwrap();
        framework
            .inject_concept_with_ttl("third", HVec10240::random(), 3600)
            .await
            .unwrap();

        // Wait for first concept to expire (1s TTL + margin for injection overhead)
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        let purged = framework.purge_expired().await.unwrap();
        assert_eq!(purged, 1, "only first concept should expire");

        // Wait for second concept to expire (3s TTL)
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        let purged = framework.purge_expired().await.unwrap();
        assert_eq!(purged, 1, "second concept should now expire");

        // Third should still exist
        let sing = framework.singularity.read().await;
        assert!(
            sing.concepts.contains_key("third"),
            "long TTL concept should remain"
        );
    }

    #[tokio::test]
    async fn test_query_in_session_filtering() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        // Inject concepts for two different sessions
        let mut meta1 = HashMap::new();
        meta1.insert("session_id".to_string(), serde_json::json!("session-1"));
        framework
            .inject_text_with_metadata("doc-1-1", "apple fruit red", meta1)
            .await
            .unwrap();

        let mut meta2 = HashMap::new();
        meta2.insert("session_id".to_string(), serde_json::json!("session-2"));
        framework
            .inject_text_with_metadata("doc-2-1", "apple fruit green", meta2)
            .await
            .unwrap();

        // Query in session 1
        let results1 = framework.query_in_session("apple", "session-1", 10).await.unwrap();
        assert_eq!(results1.len(), 1);
        assert_eq!(results1[0].0, "doc-1-1");

        // Query in session 2
        let results2 = framework.query_in_session("apple", "session-2", 10).await.unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].0, "doc-2-1");

        // Query in non-existent session
        let results3 = framework.query_in_session("apple", "session-3", 10).await.unwrap();
        assert!(results3.is_empty());
    }

    #[tokio::test]
    async fn zero_ttl_still_sets_expiration() {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        // Zero TTL should still set expires_at (immediate expiration)
        framework
            .inject_concept_with_ttl("zero-ttl", HVec10240::random(), 0)
            .await
            .unwrap();

        let sing = framework.singularity.read().await;
        let concept = sing.get("zero-ttl").unwrap();
        assert!(
            concept.expires_at.is_some(),
            "zero TTL should still set expires_at"
        );
        // expires_at should be approximately now
        let now = unix_now_secs();
        assert!(
            concept.expires_at.unwrap() <= now,
            "zero TTL should expire immediately or already be expired"
        );
    }
}
