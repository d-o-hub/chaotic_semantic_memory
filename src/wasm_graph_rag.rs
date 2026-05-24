//! WASM GraphRAG bindings.
//!
//! Split from `wasm_ext.rs` to maintain the 500-LOC project limit.

#[cfg(target_arch = "wasm32")]
use js_sys::Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::wasm::WasmFramework;
#[cfg(target_arch = "wasm32")]
use crate::wasm_utils::to_js_error;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmFramework {
    /// GraphRAG retrieval: similarity + graph traversal hybrid using vector query.
    pub async fn probe_with_graph(
        &self,
        vector: &[u8],
        anchor_top_k: usize,
        max_hops: usize,
        min_assoc_strength: f32,
        similarity_weight: f32,
        graph_weight: f32,
        final_top_k: usize,
    ) -> Result<Array, JsValue> {
        use crate::retrieval::GraphRagConfig;
        let query = crate::hyperdim::HVec10240::from_bytes(vector).map_err(to_js_error)?;
        let config = GraphRagConfig {
            anchor_top_k,
            max_hops,
            min_assoc_strength,
            similarity_weight,
            graph_weight,
            final_top_k,
        };

        let results = self
            .framework
            .probe_with_graph(query, config)
            .await
            .map_err(to_js_error)?;

        let array = Array::new();
        for r in results {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &r.id.into());
            let _ = js_sys::Reflect::set(&obj, &"score".into(), &r.score.into());
            let _ = js_sys::Reflect::set(&obj, &"similarity".into(), &r.similarity.into());
            let _ = js_sys::Reflect::set(
                &obj,
                &"anchor_id".into(),
                &r.anchor_id.map_or(JsValue::NULL, |id| id.into()),
            );
            let _ = js_sys::Reflect::set(
                &obj,
                &"hop_distance".into(),
                &(r.hop_distance as f64).into(),
            );
            let _ = js_sys::Reflect::set(&obj, &"assoc_strength".into(), &r.assoc_strength.into());
            array.push(&obj);
        }

        Ok(array)
    }

    /// GraphRAG retrieval: similarity + graph traversal hybrid using text query.
    pub async fn probe_text_with_graph(
        &self,
        text: String,
        anchor_top_k: usize,
        max_hops: usize,
        min_assoc_strength: f32,
        similarity_weight: f32,
        graph_weight: f32,
        final_top_k: usize,
    ) -> Result<Array, JsValue> {
        use crate::retrieval::GraphRagConfig;
        let config = GraphRagConfig {
            anchor_top_k,
            max_hops,
            min_assoc_strength,
            similarity_weight,
            graph_weight,
            final_top_k,
        };

        let results = self
            .framework
            .probe_text_with_graph(&text, config)
            .await
            .map_err(to_js_error)?;

        let array = Array::new();
        for r in results {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"id".into(), &r.id.into());
            let _ = js_sys::Reflect::set(&obj, &"score".into(), &r.score.into());
            let _ = js_sys::Reflect::set(&obj, &"similarity".into(), &r.similarity.into());
            let _ = js_sys::Reflect::set(
                &obj,
                &"anchor_id".into(),
                &r.anchor_id.map_or(JsValue::NULL, |id| id.into()),
            );
            let _ = js_sys::Reflect::set(
                &obj,
                &"hop_distance".into(),
                &(r.hop_distance as f64).into(),
            );
            let _ = js_sys::Reflect::set(&obj, &"assoc_strength".into(), &r.assoc_strength.into());
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
        let ns = self.framework.namespace().await;
        let config = TraversalConfig::default();
        let results = sing.bfs(&ns, &start, &config).map_err(to_js_error)?;
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
}

// Ensure tests can use the wasm_graph_rag logic on native if needed
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    // Shared tests for WASM GraphRAG patterns could go here
}
