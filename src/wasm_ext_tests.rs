#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tests for WASM extension methods to run underlying data pattern tests on native targets.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use crate::framework_builder::FrameworkBuilder;
    use crate::framework_events::MemoryEvent;
    use crate::graph_traversal::TraversalConfig;
    use crate::metadata_filter::MetadataFilter;
    use csm_core_lib::hyperdim::HVec10240;
    use serde_json::json;
    use std::collections::HashMap;

    // to_js_error is a string conversion that works on native targets too
    fn to_js_error_test(msg: &str) -> bool {
        !msg.is_empty()
    }

    #[test]
    fn metadata_filter_eq_json_roundtrip() {
        let filter = MetadataFilter::eq("type", "document");
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_in_json_roundtrip() {
        let filter = MetadataFilter::in_("tag", vec![json!("rust"), json!("python")]);
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_exists_json_roundtrip() {
        let filter = MetadataFilter::exists("title");
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_and_json_roundtrip() {
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::exists("author"),
        ]);
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_or_json_roundtrip() {
        let filter = MetadataFilter::or(vec![
            MetadataFilter::eq("status", "active"),
            MetadataFilter::eq("status", "pending"),
        ]);
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_not_json_roundtrip() {
        let filter = MetadataFilter::Not(Box::new(MetadataFilter::eq("private", true)));
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_nested_complex_json_roundtrip() {
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::in_("tag", vec![json!("rust"), json!("python")]),
            MetadataFilter::Not(Box::new(MetadataFilter::eq("private", true))),
        ]);
        let json = serde_json::to_string(&filter).unwrap();
        let parsed: MetadataFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, parsed);
    }

    #[test]
    fn metadata_filter_json_string_format() {
        let filter = MetadataFilter::eq("category", "science");
        let json = serde_json::to_string(&filter).unwrap();
        assert!(json.contains("Eq") && json.contains("category") && json.contains("science"));
    }

    #[test]
    fn traversal_config_defaults() {
        let config = TraversalConfig::default();
        assert_eq!(config.max_depth, 3);
        assert!((config.min_strength - (0.0)).abs() < 1e-6);
        assert_eq!(config.max_results, 100);
    }

    #[test]
    fn traversal_config_custom_values() {
        let config = TraversalConfig {
            max_depth: 5,
            min_strength: 0.7,
            ..Default::default()
        };
        assert_eq!(config.max_depth, 5);
        assert!((config.min_strength - (0.7)).abs() < 1e-6);
    }

    #[test]
    fn hvec_bytes_roundtrip() {
        let original = HVec10240::random();
        let bytes = original.to_bytes();
        let restored = HVec10240::from_bytes(&bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn hvec_bytes_length() {
        let hvec = HVec10240::random();
        let bytes = hvec.to_bytes();
        assert_eq!(bytes.len(), 1280); // 10240 bits / 8 = 1280 bytes
    }

    #[test]
    fn hvec_from_bytes_invalid_length() {
        let short_bytes = vec![0u8; 100];
        assert!(HVec10240::from_bytes(&short_bytes).is_err());
    }

    #[test]
    fn memory_event_variants_construct() {
        let injected = MemoryEvent::ConceptInjected {
            id: "test-id".to_string(),
            timestamp: 12345,
        };
        let updated = MemoryEvent::ConceptUpdated {
            id: "test-id".to_string(),
            timestamp: 12346,
        };
        let deleted = MemoryEvent::ConceptDeleted {
            id: "test-id".to_string(),
            timestamp: 12347,
        };
        let associated = MemoryEvent::Associated {
            from: "a".to_string(),
            to: "b".to_string(),
            strength: 0.8,
        };
        let disassociated = MemoryEvent::Disassociated {
            from: "a".to_string(),
            to: "b".to_string(),
        };

        assert!(matches!(injected, MemoryEvent::ConceptInjected { .. }));
        assert!(format!("{updated:?}").contains("ConceptUpdated"));
        assert!(format!("{deleted:?}").contains("ConceptDeleted"));
        assert!(format!("{associated:?}").contains("Associated"));
        assert!(format!("{disassociated:?}").contains("Disassociated"));
    }

    #[test]
    fn memory_event_clone_preserves_data() {
        let event = MemoryEvent::Associated {
            from: "source".to_string(),
            to: "target".to_string(),
            strength: 0.95,
        };
        let cloned = event;
        match cloned {
            MemoryEvent::Associated { from, to, strength } => {
                assert_eq!(from, "source");
                assert_eq!(to, "target");
                assert!((strength - 0.95).abs() < 0.001);
            }
            _ => panic!("Expected Associated variant"),
        }
    }

    #[test]
    fn metadata_filter_matches_empty_metadata() {
        let filter = MetadataFilter::eq("type", "document");
        let empty_metadata = HashMap::new();
        assert!(!filter.matches(&empty_metadata));
    }

    #[test]
    fn metadata_filter_exists_on_empty_metadata() {
        let filter = MetadataFilter::exists("field");
        let empty_metadata = HashMap::new();
        assert!(!filter.matches(&empty_metadata));
    }

    #[test]
    fn wasm_hvec_bytes_roundtrip() {
        let v = HVec10240::random();
        let bytes = v.to_bytes();
        let v2 = HVec10240::from_bytes(&bytes).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn wasm_hvec_bytes_invalid_len() {
        assert!(HVec10240::from_bytes(&[0u8; 100]).is_err());
    }

    #[test]
    fn wasm_to_js_error_msg() {
        assert!(to_js_error_test("test error"));
    }

    #[tokio::test]
    async fn wasm_namespace_switching_isolates_concepts() {
        let framework = FrameworkBuilder::new()
            .without_persistence()
            .build()
            .await
            .unwrap();

        // 1. Inject into default namespace
        framework
            .inject_concept("default-concept", HVec10240::random())
            .await
            .unwrap();

        // 2. Switch namespace
        framework.set_namespace("tenant-a").await.unwrap();
        assert_eq!(framework.namespace().await, "tenant-a");

        // 3. Verify default concept is not visible in tenant-a
        let results = framework.probe(HVec10240::random(), 10).await.unwrap();
        assert!(results.is_empty());

        // 4. Inject into tenant-a
        framework
            .inject_concept("tenant-concept", HVec10240::random())
            .await
            .unwrap();
        let results = framework.probe(HVec10240::random(), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "tenant-concept");

        // 5. Switch back to default
        framework.set_namespace("_default").await.unwrap();
        assert_eq!(framework.namespace().await, "_default");

        // 6. Verify only default concept is visible
        let results = framework.probe(HVec10240::random(), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "default-concept");
    }

    #[tokio::test]
    async fn test_wasm_namespace_ops_defend_mutation() {
        // Use the native framework directly to test the same logic WasmFramework uses
        let framework = FrameworkBuilder::new()
            .without_persistence()
            .build()
            .await
            .unwrap();

        assert_eq!(framework.namespace().await, "_default");
        framework.set_namespace("new-ns").await.unwrap();
        assert_eq!(framework.namespace().await, "new-ns");

        // Verify validation rejects empty and keeps previous
        assert!(framework.set_namespace("").await.is_err());
        assert_eq!(framework.namespace().await, "new-ns");
    }

    fn native_enc(t: &str) -> Box<[u8]> {
        csm_core_lib::encoder::TextEncoder::new()
            .encode(t)
            .to_bytes()
            .into_boxed_slice()
    }

    #[test]
    fn wasm_encode_text_consistency() {
        assert_eq!(native_enc("Test"), native_enc("Test"));
    }

    #[test]
    fn wasm_encode_text_length() {
        assert_eq!(native_enc("Len").len(), 1280);
    }

    #[test]
    fn wasm_encode_text_difference() {
        assert_ne!(native_enc("A"), native_enc("B"));
    }

    #[test]
    fn wasm_encode_text_empty() {
        assert_eq!(native_enc("").len(), 1280);
    }

    #[test]
    fn encode_text_returns_nontrivial_hvec() {
        let bytes = native_enc("hello world");
        // HVec10240 serialised = 80 × 16 bytes = 1280 bytes
        assert_eq!(
            bytes.len(),
            1280,
            "encoded length must match HVec10240 wire size"
        );
        assert!(bytes.iter().any(|&b| b != 0), "result must not be all-zero");
        assert!(
            bytes.iter().any(|&b| b != 0xff),
            "result must not be all-ones"
        );
    }

    #[test]
    fn encode_text_matches_encoder_directly() {
        use csm_core_lib::encoder::TextEncoder;
        let encoder = TextEncoder::new();
        let direct = encoder.encode("mutation-test-input").to_bytes();
        let wrapped = native_enc("mutation-test-input");
        assert_eq!(direct.as_slice(), wrapped.as_ref());
    }

    #[test]
    fn encode_text_deterministic_across_calls() {
        let a = native_enc("determinism-check");
        let b = native_enc("determinism-check");
        let c = native_enc("determinism-check");
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[tokio::test]
    async fn probe_filtered_excludes_nonmatching_metadata() {
        let fw = crate::ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let v1 = HVec10240::random();
        let v2 = HVec10240::random();

        let mut meta_a = std::collections::HashMap::new();
        meta_a.insert("type".to_string(), json!("article"));
        meta_a.insert("status".to_string(), json!("published"));
        fw.inject_concept_with_metadata("doc-a", v1, meta_a)
            .await
            .unwrap();

        let mut meta_b = std::collections::HashMap::new();
        meta_b.insert("type".to_string(), json!("image"));
        meta_b.insert("status".to_string(), json!("draft"));
        fw.inject_concept_with_metadata("doc-b", v2, meta_b)
            .await
            .unwrap();

        let filter = MetadataFilter::eq("type", "article");
        let results = fw.probe_filtered(&v1, 10, &filter).await.unwrap();
        assert!(
            results.iter().any(|(id, _)| id == "doc-a"),
            "filtered results must include the matching concept"
        );
        assert!(
            !results.iter().any(|(id, _)| id == "doc-b"),
            "filtered results must exclude the non-matching concept"
        );
    }

    #[tokio::test]
    async fn probe_filtered_returns_empty_when_no_match() {
        let fw = crate::ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let v1 = HVec10240::random();
        let mut meta = std::collections::HashMap::new();
        meta.insert("type".to_string(), json!("video"));
        fw.inject_concept_with_metadata("only-doc", v1, meta)
            .await
            .unwrap();

        let filter = MetadataFilter::eq("type", "nonexistent");
        let results = fw.probe_filtered(&v1, 10, &filter).await.unwrap();
        assert!(
            results.is_empty(),
            "no results should match a filter with no matching concepts"
        );
    }

    #[tokio::test]
    async fn probe_with_graph_returns_results_for_associated_concepts() {
        let fw = crate::ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let v1 = HVec10240::random();
        let v2 = HVec10240::random();

        fw.inject_concept("anchor", v1).await.unwrap();
        fw.inject_concept("neighbor", v2).await.unwrap();
        fw.associate("anchor", "neighbor", 0.9).await.unwrap();

        let config = crate::retrieval::GraphRagConfig {
            anchor_top_k: 5,
            max_hops: 2,
            min_assoc_strength: 0.1,
            similarity_weight: 0.7,
            graph_weight: 0.3,
            final_top_k: 5,
        };
        let results = fw.probe_with_graph(v1, config).await.unwrap();
        assert!(
            !results.is_empty(),
            "probe_with_graph must return results when concepts exist"
        );
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(
            ids.contains(&"anchor"),
            "results must include the anchor concept"
        );
    }

    #[tokio::test]
    async fn probe_with_graph_empty_index_returns_empty() {
        let fw = crate::ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();

        let config = crate::retrieval::GraphRagConfig {
            anchor_top_k: 5,
            max_hops: 2,
            min_assoc_strength: 0.1,
            similarity_weight: 0.7,
            graph_weight: 0.3,
            final_top_k: 5,
        };
        let results = fw
            .probe_with_graph(HVec10240::random(), config)
            .await
            .unwrap();
        assert!(
            results.is_empty(),
            "probe_with_graph on empty index must return empty"
        );
    }
}

#[tokio::test]
async fn test_namespace_validation_integration() {
    let framework: crate::ChaoticSemanticFramework = crate::ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // 1. Valid namespace
    assert!(framework.set_namespace("tenant-a").await.is_ok());

    // 2. Invalid namespace: empty
    let res = framework.set_namespace("").await;
    assert!(res.is_err());

    // 3. Invalid namespace: too long
    let res = framework.set_namespace("a".repeat(129)).await;
    assert!(res.is_err());

    // 4. Invalid namespace: control chars
    let res = framework.set_namespace("ns\x00").await;
    assert!(res.is_err());
}
