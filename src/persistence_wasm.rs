//! Persistence stub for wasm32 builds.
//!
//! Canonical stub lives in `csm-persistence` (ADR-0094); this module keeps the
//! root public path (`chaotic_semantic_memory::persistence_wasm`, re-exported as
//! `persistence` on wasm32) and re-exports the owner's type.

pub use csm_memory::ConceptVersion;
pub use csm_persistence::Persistence;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use csm_core_lib::hyperdim::HVec10240;

    #[tokio::test]
    async fn new_local_returns_unsupported() {
        let result = Persistence::new_local("test.db").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));
    }

    #[tokio::test]
    async fn new_turso_returns_unsupported() {
        let result = Persistence::new_turso("https://example.com", "token").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));
    }

    #[tokio::test]
    async fn load_associations_returns_unsupported() {
        let p = Persistence {};
        let result = p.load_associations("test_ns", "test_id").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));
    }

    #[tokio::test]
    async fn load_all_associations_returns_unsupported() {
        let p = Persistence {};
        let result = p.load_all_associations("test_ns").await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));
    }

    #[tokio::test]
    async fn all_operations_return_unsupported() {
        // Persistence struct cannot be created (constructor fails),
        // but we can verify the error function behavior
        let err = wasm_persistence_unavailable();

        assert!(matches!(err, MemoryError::UnsupportedOperation(_)));

        // Verify error message contains expected text
        let msg = err.to_string();
        assert!(msg.contains("wasm32"));
        assert!(msg.contains("unavailable"));
    }

    #[test]
    fn concept_version_serialization_roundtrip() {
        // ConceptVersion is used in WASM builds for version serialization
        let version = ConceptVersion {
            concept_id: "test-id".to_string(),
            version: 1,
            timestamp_unix: 12345,
            vector: Some(HVec10240::zero()),
            metadata: Some(serde_json::json!({"key": "value"})),
            vector_changed: None,
            metadata_changed: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&version).unwrap();

        // Deserialize back
        let recovered: ConceptVersion = serde_json::from_str(&json).unwrap();

        assert_eq!(recovered, version);
    }
}
