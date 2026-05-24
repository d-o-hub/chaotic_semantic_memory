//! WASM extension methods: stats, text encoding, graph traversal, metadata ops.
//!
//! Split from `wasm.rs` to keep each file under the 500-LOC project limit.

// Redundant clones are intentional for WASM ownership semantics

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
        let ns = self.framework.namespace().await;
        Ok(sing.len(&ns))
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
        let ns = self.framework.namespace().await;
        sing.clear_associations(&ns, &id).map_err(to_js_error)
    }

    /// Get direct neighbors of a concept with edge strengths.
    ///
    /// Returns an Array of `{to: string, strength: number}` objects.
    pub async fn neighbors(&self, id: String, min_strength: f32) -> Result<Array, JsValue> {
        let sing = self.framework.singularity.read().await;
        let ns = self.framework.namespace().await;
        let neighbors = sing.neighbors(&ns, &id, min_strength);
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

    /// List all historical versions of a concept.
    #[wasm_bindgen(js_name = listVersions)]
    pub async fn list_versions(&self, id: String) -> Result<Array, JsValue> {
        let versions = self
            .framework
            .list_versions(&id)
            .await
            .map_err(to_js_error)?;
        let array = Array::new();
        for v in versions {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"conceptId".into(), &v.concept_id.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"version".into(), &(v.version as u32).into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(
                &obj,
                &"timestampUnix".into(),
                &(v.timestamp_unix as f64).into(),
            )
            .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            if let Some(vc) = v.vector_changed {
                js_sys::Reflect::set(&obj, &"vectorChanged".into(), &vc.into())
                    .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            }
            if let Some(mc) = v.metadata_changed {
                js_sys::Reflect::set(&obj, &"metadataChanged".into(), &mc.into())
                    .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            }
            array.push(&obj);
        }
        Ok(array)
    }

    /// Load a specific concept version.
    #[wasm_bindgen(js_name = getVersion)]
    pub async fn get_version(&self, id: String, version: u32) -> Result<JsValue, JsValue> {
        let concept_opt = self
            .framework
            .get_version(&id, version as u64)
            .await
            .map_err(to_js_error)?;
        match concept_opt {
            Some(concept) => crate::wasm::concept_to_js_value(&concept),
            None => Ok(JsValue::NULL),
        }
    }

    /// Roll back a concept to a historical version.
    #[wasm_bindgen(js_name = rollbackToVersion)]
    pub async fn rollback_to_version(&self, id: String, version: u32) -> Result<JsValue, JsValue> {
        let concept = self
            .framework
            .rollback_to_version(&id, version as u64)
            .await
            .map_err(to_js_error)?;
        crate::wasm::concept_to_js_value(&concept)
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
