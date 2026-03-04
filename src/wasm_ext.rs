//! WASM extension methods: stats, text encoding, graph traversal, metadata ops.
//!
//! Split from `wasm.rs` to keep each file under the 500-LOC project limit.

use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::wasm::{WasmFramework, to_js_error};

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
        let mut sing = self.framework.singularity.write().await;
        sing.update_metadata(&id, metadata).map_err(to_js_error)
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
    /// Uses default `TraversalConfig` (max_depth=3, max_results=100).
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
    /// Uses default `TraversalConfig` (max_depth=3).
    pub async fn shortest_path(&self, from: String, to: String) -> Result<Array, JsValue> {
        use crate::graph_traversal::TraversalConfig;
        let sing = self.framework.singularity.read().await;
        let config = TraversalConfig::default();
        let path = sing
            .shortest_path(&from, &to, &config)
            .map_err(to_js_error)?;
        let array = Array::new();
        if let Some(nodes) = path {
            for id in nodes {
                array.push(&id.into());
            }
        }
        Ok(array)
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
