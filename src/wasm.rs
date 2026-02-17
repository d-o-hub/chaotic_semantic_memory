//! WASM bindings for chaotic semantic memory

use js_sys::Array;
use wasm_bindgen::prelude::*;

use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;

/// WASM-friendly wrapper for the framework
#[wasm_bindgen]
pub struct WasmFramework {
    framework: ChaoticSemanticFramework,
}

#[wasm_bindgen]
impl WasmFramework {
    /// Create a new framework instance (no persistence in WASM)
    pub async fn new() -> Result<WasmFramework, JsValue> {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        Ok(WasmFramework { framework })
    }

    /// Inject a concept
    pub async fn inject_concept(&self, id: String, vector: &[u8]) -> Result<(), JsValue> {
        let hvec =
            HVec10240::from_bytes(vector).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        self.framework
            .inject_concept(id, hvec)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Query for similar concepts
    pub async fn probe(&self, vector: &[u8], top_k: usize) -> Result<Array, JsValue> {
        let hvec =
            HVec10240::from_bytes(vector).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let results = self
            .framework
            .probe(hvec, top_k)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let array = Array::new();
        for (id, score) in results {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"id".into(), &id.into()).unwrap();
            js_sys::Reflect::set(&obj, &"score".into(), &score.into()).unwrap();
            array.push(&obj);
        }

        Ok(array)
    }

    /// Associate two concepts
    pub async fn associate(&self, from: String, to: String, strength: f32) -> Result<(), JsValue> {
        self.framework
            .associate(&from, &to, strength)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Delete concept by ID
    pub async fn delete_concept(&self, id: String) -> Result<(), JsValue> {
        self.framework
            .delete_concept(&id)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    /// Get associations for a concept
    pub async fn get_associations(&self, id: String) -> Result<Array, JsValue> {
        let associations = self
            .framework
            .get_associations(&id)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let array = Array::new();
        for (to, strength) in associations {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"to".into(), &to.into()).unwrap();
            js_sys::Reflect::set(&obj, &"strength".into(), &strength.into()).unwrap();
            array.push(&obj);
        }
        Ok(array)
    }

    /// Get framework metrics snapshot
    pub fn metrics_snapshot(&self) -> JsValue {
        let metrics = self.framework.metrics_snapshot();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"concepts_injected_total".into(),
            &(metrics.concepts_injected_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"associations_created_total".into(),
            &(metrics.associations_created_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"probes_total".into(),
            &(metrics.probes_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"avg_probe_latency_ms".into(),
            &metrics.avg_probe_latency_ms.into(),
        )
        .unwrap();
        obj.into()
    }

    /// Get framework stats
    pub async fn stats(&self) -> Result<JsValue, JsValue> {
        let stats = self
            .framework
            .stats()
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"concept_count".into(),
            &(stats.concept_count as u32).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"db_size_bytes".into(),
            &(stats.db_size_bytes as f64).into(),
        )
        .unwrap();

        Ok(obj.into())
    }
}

/// Create a random hypervector (1280 bytes)
#[wasm_bindgen]
pub fn random_hypervector() -> Box<[u8]> {
    HVec10240::random().to_bytes().into_boxed_slice()
}

/// Compute cosine similarity between two hypervectors
#[wasm_bindgen]
pub fn cosine_similarity(a: &[u8], b: &[u8]) -> Result<f32, JsValue> {
    let hvec_a = HVec10240::from_bytes(a).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    let hvec_b = HVec10240::from_bytes(b).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    Ok(hvec_a.cosine_similarity(&hvec_b))
}
