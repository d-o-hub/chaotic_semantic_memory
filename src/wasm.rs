//! WASM bindings for chaotic semantic memory

use js_sys::{Array, Float32Array, Uint8Array};
use tracing::warn;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::export_payload::{unix_now_secs, ExportPayload};
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
        let hvec = HVec10240::from_bytes(vector).map_err(to_js_error)?;

        self.framework
            .inject_concept(id, hvec)
            .await
            .map_err(to_js_error)
    }

    /// Query for similar concepts
    pub async fn probe(&self, vector: &[u8], top_k: usize) -> Result<Array, JsValue> {
        let hvec = HVec10240::from_bytes(vector).map_err(to_js_error)?;

        let results = self
            .framework
            .probe(hvec, top_k)
            .await
            .map_err(to_js_error)?;

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
            .map_err(to_js_error)
    }

    /// Delete concept by ID
    pub async fn delete_concept(&self, id: String) -> Result<(), JsValue> {
        self.framework
            .delete_concept(&id)
            .await
            .map_err(to_js_error)
    }

    /// Get associations for a concept
    pub async fn get_associations(&self, id: String) -> Result<Array, JsValue> {
        let associations = self
            .framework
            .get_associations(&id)
            .await
            .map_err(to_js_error)?;

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
    pub async fn metrics_snapshot(&self) -> JsValue {
        let metrics = self.framework.metrics_snapshot().await;
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
        js_sys::Reflect::set(
            &obj,
            &"cache_hits_total".into(),
            &(metrics.cache_hits_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"cache_misses_total".into(),
            &(metrics.cache_misses_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"cache_evictions_total".into(),
            &(metrics.cache_evictions_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"reservoir_steps_total".into(),
            &(metrics.reservoir_steps_total as f64).into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"avg_reservoir_step_latency_us".into(),
            &metrics.avg_reservoir_step_latency_us.into(),
        )
        .unwrap();
        js_sys::Reflect::set(
            &obj,
            &"reservoir_nodes_active".into(),
            &(metrics.reservoir_nodes_active as f64).into(),
        )
        .unwrap();
        obj.into()
    }

    /// Process a temporal sequence and return the resulting hypervector bytes.
    #[wasm_bindgen(js_name = processSequence)]
    pub async fn process_sequence(&self, sequence: Array) -> Result<Box<[u8]>, JsValue> {
        let mut parsed_sequence = Vec::with_capacity(sequence.length() as usize);
        for item in sequence.iter() {
            let step = item
                .dyn_into::<Float32Array>()
                .map_err(|_| JsValue::from_str("processSequence expects Float32Array items"))?;
            parsed_sequence.push(step.to_vec());
        }

        let output = self
            .framework
            .process_sequence(&parsed_sequence)
            .await
            .map_err(to_js_error)?;
        Ok(output.to_bytes().into_boxed_slice())
    }

    /// Export all concepts and associations to bytes for in-browser storage.
    #[wasm_bindgen(js_name = exportToBytes)]
    pub async fn export_to_bytes(&self) -> Result<Uint8Array, JsValue> {
        let payload = {
            let singularity = self.framework.singularity.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: singularity.all_concepts(),
                associations: singularity.all_associations(),
            }
        };

        let data = bincode::serialize(&payload).map_err(to_js_error)?;
        Ok(Uint8Array::from(data.as_slice()))
    }

    /// Import state from bytes previously produced by `exportToBytes`.
    #[wasm_bindgen(js_name = importFromBytes)]
    pub async fn import_from_bytes(&self, data: Uint8Array, merge: bool) -> Result<usize, JsValue> {
        let payload: ExportPayload = bincode::deserialize(&data.to_vec()).map_err(to_js_error)?;

        if !merge {
            let mut singularity = self.framework.singularity.write().await;
            singularity.clear();
        }

        let mut singularity = self.framework.singularity.write().await;
        for concept in &payload.concepts {
            self.framework
                .validate_concept(concept)
                .map_err(to_js_error)?;
            singularity.inject(concept.clone()).map_err(to_js_error)?;
        }

        for (from, to, strength) in &payload.associations {
            if let Err(error) = singularity.associate(from, to, *strength) {
                warn!(
                    from_id = %from,
                    to_id = %to,
                    strength = *strength,
                    error = %error,
                    "skipping invalid association during wasm import"
                );
            }
        }

        Ok(payload.concepts.len())
    }

    /// Get framework stats
    pub async fn stats(&self) -> Result<JsValue, JsValue> {
        let stats = self.framework.stats().await.map_err(to_js_error)?;

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
    let hvec_a = HVec10240::from_bytes(a).map_err(to_js_error)?;
    let hvec_b = HVec10240::from_bytes(b).map_err(to_js_error)?;

    Ok(hvec_a.cosine_similarity(&hvec_b))
}

fn to_js_error<E: std::fmt::Display>(error: E) -> JsValue {
    JsValue::from_str(&error.to_string())
}
