//! WASM bindings for chaotic semantic memory

use bincode::Options;
use js_sys::{Array, Float32Array, Uint8Array};
use tracing::warn;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

pub(crate) use crate::export_payload::{BinaryExportPayload, ExportPayload, unix_now_secs};
pub(crate) use crate::framework::{ChaoticSemanticFramework, MAX_IMPORT_SIZE};
pub(crate) use crate::hyperdim::HVec10240;
pub(crate) use crate::wasm_ext::{concept_to_js_value, to_js_error};

#[wasm_bindgen(start)]
pub fn initialize_wasm() {
    console_error_panic_hook::set_once();
}

/// WASM-friendly wrapper for the framework
#[wasm_bindgen]
pub struct WasmFramework {
    pub(crate) framework: ChaoticSemanticFramework,
}

#[wasm_bindgen]
impl WasmFramework {
    /// Create a new framework instance (no persistence in WASM)
    pub async fn new() -> Result<WasmFramework, JsValue> {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .map_err(to_js_error)?;

        Ok(WasmFramework { framework })
    }

    /// Set the current namespace
    #[wasm_bindgen(js_name = setNamespace)]
    pub async fn set_namespace(&self, ns: String) {
        self.framework.set_namespace(ns).await;
    }

    /// Get the current namespace
    #[wasm_bindgen(js_name = getNamespace)]
    pub async fn get_namespace(&self) -> String {
        self.framework.namespace().await
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
            js_sys::Reflect::set(&obj, &"id".into(), &id.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"score".into(), &score.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
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

    /// Update a concept's vector
    pub async fn update_concept(&self, id: String, vector: &[u8]) -> Result<(), JsValue> {
        let hvec = HVec10240::from_bytes(vector).map_err(to_js_error)?;
        self.framework
            .update_concept_vector(&id, hvec)
            .await
            .map_err(to_js_error)
    }

    /// Remove an association between two concepts
    pub async fn disassociate(&self, from: String, to: String) -> Result<(), JsValue> {
        self.framework
            .disassociate(&from, &to)
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
            js_sys::Reflect::set(&obj, &"to".into(), &to.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            js_sys::Reflect::set(&obj, &"strength".into(), &strength.into())
                .map_err(|_| JsValue::from_str("failed to set JS property"))?;
            array.push(&obj);
        }
        Ok(array)
    }

    /// Get a concept by ID
    pub async fn get_concept(&self, id: String) -> Result<JsValue, JsValue> {
        let concept_opt = self.framework.get_concept(&id).await.map_err(to_js_error)?;

        match concept_opt {
            Some(concept) => concept_to_js_value(&concept),
            None => Ok(JsValue::NULL),
        }
    }

    /// Inject multiple concepts in batch
    pub async fn inject_concepts(&self, ids: Array, vectors: Array) -> Result<(), JsValue> {
        self.framework
            .validate_batch_size(ids.length() as usize)
            .map_err(to_js_error)?;

        if ids.length() != vectors.length() {
            return Err(JsValue::from_str(
                "ids and vectors arrays must have the same length",
            ));
        }

        for i in 0..ids.length() {
            let id = ids
                .get(i)
                .as_string()
                .ok_or_else(|| JsValue::from_str("id must be a string"))?;
            let vector_bytes = vectors
                .get(i)
                .dyn_into::<Uint8Array>()
                .map_err(|_| JsValue::from_str("vector must be Uint8Array"))?
                .to_vec();
            let hvec = HVec10240::from_bytes(&vector_bytes).map_err(to_js_error)?;

            self.framework
                .inject_concept(id, hvec)
                .await
                .map_err(to_js_error)?;
        }

        Ok(())
    }

    /// Create multiple associations in batch
    pub async fn associate_many(&self, associations: Array) -> Result<(), JsValue> {
        self.framework
            .validate_batch_size(associations.length() as usize)
            .map_err(to_js_error)?;

        for i in 0..associations.length() {
            let assoc = associations.get(i);
            let from = js_sys::Reflect::get(&assoc, &"from".into())
                .map_err(|_| JsValue::from_str("association must have 'from' field"))?
                .as_string()
                .ok_or_else(|| JsValue::from_str("'from' must be a string"))?;
            let to = js_sys::Reflect::get(&assoc, &"to".into())
                .map_err(|_| JsValue::from_str("association must have 'to' field"))?
                .as_string()
                .ok_or_else(|| JsValue::from_str("'to' must be a string"))?;
            let strength = js_sys::Reflect::get(&assoc, &"strength".into())
                .map_err(|_| JsValue::from_str("association must have 'strength' field"))?
                .as_f64()
                .ok_or_else(|| JsValue::from_str("'strength' must be a number"))?
                as f32;

            self.framework
                .associate(&from, &to, strength)
                .await
                .map_err(to_js_error)?;
        }

        Ok(())
    }

    /// Probe for similar concepts with multiple queries in batch
    pub async fn probe_batch(&self, vectors: Array, top_k: usize) -> Result<Array, JsValue> {
        self.framework
            .validate_batch_size(vectors.length() as usize)
            .map_err(to_js_error)?;

        let results = Array::new();

        for i in 0..vectors.length() {
            let vector_bytes = vectors
                .get(i)
                .dyn_into::<Uint8Array>()
                .map_err(|_| JsValue::from_str("vector must be Uint8Array"))?
                .to_vec();
            let hvec = HVec10240::from_bytes(&vector_bytes).map_err(to_js_error)?;

            let query_results = self
                .framework
                .probe(hvec, top_k)
                .await
                .map_err(to_js_error)?;

            let query_array = Array::new();
            for (id, score) in query_results {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"id".into(), &id.into())
                    .map_err(|_| JsValue::from_str("failed to set JS property"))?;
                js_sys::Reflect::set(&obj, &"score".into(), &score.into())
                    .map_err(|_| JsValue::from_str("failed to set JS property"))?;
                query_array.push(&obj);
            }
            results.push(&query_array);
        }

        Ok(results)
    }

    /// Get framework metrics snapshot
    pub async fn metrics_snapshot(&self) -> Result<JsValue, JsValue> {
        let metrics = self.framework.metrics_snapshot().await;
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(
            &obj,
            &"concepts_injected_total".into(),
            &(metrics.concepts_injected_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"associations_created_total".into(),
            &(metrics.associations_created_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"probes_total".into(),
            &(metrics.probes_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"avg_probe_latency_ms".into(),
            &metrics.avg_probe_latency_ms.into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"cache_hits_total".into(),
            &(metrics.cache_hits_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"cache_misses_total".into(),
            &(metrics.cache_misses_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"cache_evictions_total".into(),
            &(metrics.cache_evictions_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"reservoir_steps_total".into(),
            &(metrics.reservoir_steps_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"avg_reservoir_step_latency_us".into(),
            &metrics.avg_reservoir_step_latency_us.into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"reservoir_nodes_active".into(),
            &(metrics.reservoir_nodes_active as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"persist_ops_total".into(),
            &(metrics.persist_ops_total as f64).into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        js_sys::Reflect::set(
            &obj,
            &"avg_persist_latency_ms".into(),
            &metrics.avg_persist_latency_ms.into(),
        )
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
        Ok(obj.into())
    }

    /// Process a temporal sequence and return the resulting hypervector bytes.
    #[wasm_bindgen(js_name = processSequence)]
    pub async fn process_sequence(&self, sequence: Array) -> Result<Box<[u8]>, JsValue> {
        self.framework
            .validate_sequence_length(sequence.length() as usize)
            .map_err(to_js_error)?;

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
            let ns = self.framework.namespace().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: singularity.all_concepts(&ns),
                associations: singularity.all_associations(&ns),
            }
        };

        // Use BinaryExportPayload for bincode compatibility (serde_json::Value is incompatible with bincode)
        let binary_payload = BinaryExportPayload::from(payload);
        let data = bincode::serialize(&binary_payload).map_err(to_js_error)?;
        Ok(Uint8Array::from(data.as_slice()))
    }

    /// Import state from bytes previously produced by `exportToBytes`.
    #[wasm_bindgen(js_name = importFromBytes)]
    pub async fn import_from_bytes(&self, data: Uint8Array, merge: bool) -> Result<usize, JsValue> {
        let bytes = data.to_vec();

        if bytes.len() > MAX_IMPORT_SIZE as usize {
            return Err(JsValue::from_str(&format!(
                "Import data size {} exceeds maximum allowed size {}",
                bytes.len(),
                MAX_IMPORT_SIZE
            )));
        }

        // Deserialize as BinaryExportPayload (bincode-compatible), then convert to ExportPayload
        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let binary_payload: BinaryExportPayload =
            options.deserialize(&bytes).map_err(to_js_error)?;
        let payload = binary_payload.to_export_payload().map_err(to_js_error)?;

        let ns = self.framework.namespace().await;

        if !merge {
            let mut singularity = self.framework.singularity.write().await;
            singularity.clear(&ns);
        }

        let mut singularity = self.framework.singularity.write().await;
        for concept in &payload.concepts {
            self.framework
                .validate_concept(concept)
                .map_err(to_js_error)?;
            singularity
                .inject(&ns, concept.clone())
                .map_err(to_js_error)?;
        }

        for (from, to, strength) in &payload.associations {
            if let Err(error) = singularity.associate(&ns, from, to, *strength) {
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
}

#[wasm_bindgen]
pub fn random_hypervector() -> Box<[u8]> {
    HVec10240::random().to_bytes().into_boxed_slice()
}

#[wasm_bindgen]
pub fn encode_text(text: &str) -> Box<[u8]> {
    let encoder = crate::encoder::TextEncoder::new();
    encoder.encode(text).to_bytes().into_boxed_slice()
}

#[wasm_bindgen]
pub fn cosine_similarity(a: &[u8], b: &[u8]) -> Result<f32, JsValue> {
    let hvec_a = HVec10240::from_bytes(a).map_err(to_js_error)?;
    let hvec_b = HVec10240::from_bytes(b).map_err(to_js_error)?;

    Ok(hvec_a.cosine_similarity(&hvec_b))
}

#[wasm_bindgen(typescript_custom_section)]
const TS: &'static str = r#"
export interface ProbeResult { id: string; score: number; }
export interface AssociationResult { to: string; strength: number; }
"#;
