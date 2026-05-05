//! Tests for WASM extension methods: verify data patterns on native targets.

#![allow(clippy::float_cmp)] // Exact float comparisons for test assertions

use crate::framework_builder::FrameworkBuilder;
use crate::framework_events::MemoryEvent;
use crate::graph_traversal::TraversalConfig;
use crate::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
use serde_json::json;
use std::collections::HashMap;

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
    assert_eq!(config.min_strength, 0.0);
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
    assert_eq!(config.min_strength, 0.7);
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

    assert!(matches!(
        injected.clone(),
        MemoryEvent::ConceptInjected { .. }
    ));
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
    let cloned = event.clone();
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
fn builder_with_namespace_sets_namespace() {
    // Verify that FrameworkBuilder::with_namespace correctly sets the namespace,
    // which is the underlying mechanism for WasmFramework::with_namespace.
    let builder = FrameworkBuilder::new().with_namespace("test-ns");
    assert_eq!(builder.namespace, "test-ns");
}

#[test]
fn builder_default_namespace() {
    let builder = FrameworkBuilder::new();
    assert_eq!(builder.namespace, "_default");
}
