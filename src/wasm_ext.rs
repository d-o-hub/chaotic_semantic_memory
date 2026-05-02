//! WASM extension methods: stats, text encoding, graph traversal, metadata ops.
//!
//! Split from `wasm.rs` to keep each file under the 500-LOC project limit.

// Redundant clones are intentional for WASM ownership semantics
#![allow(clippy::redundant_clone)]

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Function};
#[cfg(target_arch = "wasm32")]
use tokio::sync::broadcast::error::RecvError;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
use crate::wasm::{WasmFramework, to_js_error};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmFramework {
    /// Get framework stats
    pub async fn stats(&self) -> Result<JsValue, JsValue> {
        let stats = self.framework.stats().await.map_err(to_js_error)?;

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"concept_count".into(),
            &(stats.concept_count as u32).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"db_size_bytes".into(),
            &stats
                .db_size_bytes
                .map_or(JsValue::NULL, |v| (v as f64).into()),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;

        Ok(obj.into())
    }

    /// Get concept count (convenience method)
    pub async fn concept_count(&self) -> Result<usize, JsValue> {
        let sing = self.framework.singularity.read().await;
        Ok(sing.len())
    }

    /// Update a concept's metadata from a JSON string.
    ///
    /// The `metadata_json` argument must be a valid JSON object string,
    /// e.g. `{"category":"science","score":0.9}`.
    ///
    /// Note: In WASM, persistence is in-memory only. Use `exportToBytes` to
    /// snapshot state to IndexedDB or other storage.
    pub async fn update_concept_metadata(
        &self,
        id: String,
        metadata_json: String,
    ) -> Result<(), JsValue> {
        let metadata: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(&metadata_json)
                .map_err(|e| JsValue::from_str(&format!("invalid metadata JSON: {e}")))?;
        self.framework
            .update_concept_metadata(&id, metadata)
            .await
            .map_err(to_js_error)
    }

    /// Clear all outbound associations for a concept.
    pub async fn clear_associations(&self, id: String) -> Result<(), JsValue> {
        let mut sing = self.framework.singularity.write().await;
        sing.clear_associations(&id).map_err(to_js_error)
    }

    /// Get direct neighbors of a concept with edge strengths.
    ///
    /// Returns an Array of `{to: string, strength: number}` objects.
    pub async fn neighbors(&self, id: String, min_strength: f32) -> Result<Array, JsValue> {
        let sing = self.framework.singularity.read().await;
        let neighbors = sing.neighbors(&id, min_strength);
        let array = Array::new();
        for (to, strength) in neighbors {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"to".into(), &to.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"strength".into(), &strength.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            array.push(&obj);
        }
        Ok(array)
    }

    /// Breadth-first traversal from a starting concept.
    ///
    /// Returns an Array of `{id: string, depth: number}` objects.
    /// Uses default `TraversalConfig`.
    pub async fn bfs(&self, start: String) -> Result<Array, JsValue> {
        use crate::graph_traversal::TraversalConfig;
        let sing = self.framework.singularity.read().await;
        let config = TraversalConfig::default();
        let results = sing.bfs(&start, &config).map_err(to_js_error)?;
        let array = Array::new();
        for (id, depth) in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &id.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"depth".into(), &(depth as f64).into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            array.push(&obj);
        }
        Ok(array)
    }

    /// Find the minimum-cost path between two concepts (weighted Dijkstra).
    ///
    /// Returns an Array of concept ID strings, or an empty Array if no path exists.
    /// Uses default `TraversalConfig`.
    pub async fn shortest_path(&self, from: String, to: String) -> Result<Array, JsValue> {
        let path = self
            .framework
            .shortest_path(&from, &to)
            .await
            .map_err(to_js_error)?;
        let array = Array::new();
        if let Some(nodes) = path {
            for id in nodes {
                array.push(&id.into());
            }
        }
        Ok(array)
    }

    /// Probe for similar concepts with metadata filtering.
    pub async fn probe_filtered(
        &self,
        vector: &[u8],
        top_k: usize,
        filter_json: String,
    ) -> Result<Array, JsValue> {
        let query = crate::hyperdim::HVec10240::from_bytes(vector).map_err(to_js_error)?;
        let filter: crate::metadata_filter::MetadataFilter = serde_json::from_str(&filter_json)
            .map_err(|e| JsValue::from_str(&format!("invalid filter JSON: {e}")))?;

        let results = self
            .framework
            .probe_filtered(&query, top_k, &filter)
            .await
            .map_err(to_js_error)?;

        let array = Array::new();
        for (id, score) in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &id.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"score".into(), &score.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            array.push(&obj);
        }

        Ok(array)
    }

    /// Breadth-first traversal from a starting concept with custom config.
    pub async fn traverse(
        &self,
        start: String,
        max_depth: u32,
        min_strength: f32,
    ) -> Result<Array, JsValue> {
        let mut config = crate::graph_traversal::TraversalConfig::default();
        config.max_depth = max_depth as usize;
        config.min_strength = min_strength;

        let results = self
            .framework
            .traverse(&start, config)
            .await
            .map_err(to_js_error)?;

        let array = Array::new();
        for (id, depth) in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &id.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"depth".into(), &(depth as f64).into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            array.push(&obj);
        }
        Ok(array)
    }

    /// Register a callback for memory events.
    pub fn on_event(&self, callback: Function) {
        let mut receiver = self.framework.subscribe();
        spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let js_event = memory_event_to_js_value(&event);
                        let _ = callback.call1(&JsValue::NULL, &js_event);
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }

    /// Inject a concept from text
    pub async fn inject_text(&self, id: String, text: String) -> Result<(), JsValue> {
        self.framework
            .inject_text(&id, &text)
            .await
            .map_err(to_js_error)
    }

    /// Probe for similar concepts using text
    pub async fn probe_text(&self, query: String, top_k: usize) -> Result<Array, JsValue> {
        let results = self
            .framework
            .probe_text(&query, top_k)
            .await
            .map_err(to_js_error)?;

        let array = Array::new();
        for (id, similarity) in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &id.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"similarity".into(), &similarity.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            array.push(&obj);
        }

        Ok(array)
    }
}

#[cfg(target_arch = "wasm32")]
fn memory_event_to_js_value(event: &crate::framework_events::MemoryEvent) -> JsValue {
    let obj = js_sys::Object::new();
    match event {
        crate::framework_events::MemoryEvent::ConceptInjected { id, timestamp } => {
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &"ConceptInjected".into());
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &id.clone().into());
            let _ = js_sys::Reflect::set(&obj, &"timestamp".into(), &(*timestamp as f64).into());
        }
        crate::framework_events::MemoryEvent::ConceptUpdated { id, timestamp } => {
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &"ConceptUpdated".into());
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &id.clone().into());
            let _ = js_sys::Reflect::set(&obj, &"timestamp".into(), &(*timestamp as f64).into());
        }
        crate::framework_events::MemoryEvent::ConceptDeleted { id, timestamp } => {
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &"ConceptDeleted".into());
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &id.clone().into());
            let _ = js_sys::Reflect::set(&obj, &"timestamp".into(), &(*timestamp as f64).into());
        }
        crate::framework_events::MemoryEvent::Associated { from, to, strength } => {
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &"Associated".into());
            let _ = js_sys::Reflect::set(&obj, &"from".into(), &from.clone().into());
            let _ = js_sys::Reflect::set(&obj, &"to".into(), &to.clone().into());
            let _ = js_sys::Reflect::set(&obj, &"strength".into(), &(*strength as f64).into());
        }
        crate::framework_events::MemoryEvent::Disassociated { from, to } => {
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &"Disassociated".into());
            let _ = js_sys::Reflect::set(&obj, &"from".into(), &from.clone().into());
            let _ = js_sys::Reflect::set(&obj, &"to".into(), &to.clone().into());
        }
    }
    obj.into()
}

// TESTS - verify WASM binding data patterns on native targets

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparisons for test assertions

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
}
