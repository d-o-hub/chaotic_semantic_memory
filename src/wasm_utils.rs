//! WASM utility functions for chaotic semantic memory

use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export interface ProbeResult {
  id: string;
  score: number;
}

export interface AssociationResult {
  to: string;
  strength: number;
}

export interface FrameworkMetrics {
  concepts_injected_total: number;
  associations_created_total: number;
  probes_total: number;
  avg_probe_latency_ms: number;
  cache_hits_total: number;
  cache_misses_total: number;
  cache_evictions_total: number;
  reservoir_steps_total: number;
  avg_reservoir_step_latency_us: number;
  reservoir_nodes_active: number;
}

export interface FrameworkStats {
  concept_count: number;
  db_size_bytes: number | null;
}

export interface Concept {
  id: string;
  vector: Uint8Array;
  metadata: Record<string, any>;
  created_at: number;
  modified_at: number;
  expires_at: number | null;
  canonical_concept_ids: string[];
}

export interface VersionInfo {
  version: number;
  timestampUnix: number;
  vectorChanged: boolean;
  metadataChanged: boolean;
}

export interface GraphProbeResult {
  id: string;
  score: number;
  similarity: number;
  anchor_id: string | null;
  hop_distance: number;
  assoc_strength: number;
}

export interface TraversalResult {
  id: string;
  depth: number;
}

export type MemoryEvent =
  | { type: "ConceptInjected"; id: string; timestamp: number }
  | { type: "ConceptUpdated"; id: string; timestamp: number }
  | { type: "ConceptDeleted"; id: string; timestamp: number }
  | { type: "Associated"; from: string; to: string; strength: number }
  | { type: "Disassociated"; from: string; to: string };
"#;

/// Create a random hypervector (1280 bytes)
#[wasm_bindgen]
pub fn random_hypervector() -> Box<[u8]> {
    HVec10240::random().to_bytes().into_boxed_slice()
}

/// Encode text to a hypervector using HDC encoding
#[wasm_bindgen]
pub fn encode_text(text: &str) -> Box<[u8]> {
    let encoder = crate::encoder::TextEncoder::new();
    encoder.encode(text).to_bytes().into_boxed_slice()
}

/// Compute cosine similarity between two hypervectors
#[wasm_bindgen]
pub fn cosine_similarity(a: &[u8], b: &[u8]) -> Result<f32, JsValue> {
    let hvec_a = HVec10240::from_bytes(a).map_err(to_js_error)?;
    let hvec_b = HVec10240::from_bytes(b).map_err(to_js_error)?;

    Ok(hvec_a.cosine_similarity(&hvec_b))
}

/// Convert a Concept to a JsValue object
pub(crate) fn concept_to_js_value(concept: &Concept) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &"id".into(), &concept.id.clone().into())
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    js_sys::Reflect::set(
        &obj,
        &"vector".into(),
        &Uint8Array::from(concept.vector.to_bytes().as_slice()),
    )
    .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    // Convert metadata HashMap to JS object
    let metadata_obj = js_sys::Object::new();
    for (key, value) in &concept.metadata {
        let value_str = serde_json::to_string(value).map_err(to_js_error)?;
        let js_value = js_sys::JSON::parse(&value_str)
            .map_err(|_| JsValue::from_str("failed to parse metadata JSON"))?;
        js_sys::Reflect::set(&metadata_obj, &key.clone().into(), &js_value)
            .map_err(|_| JsValue::from_str("failed to set JS property"))?;
    }
    js_sys::Reflect::set(&obj, &"metadata".into(), &metadata_obj.into())
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    js_sys::Reflect::set(
        &obj,
        &"created_at".into(),
        &(concept.created_at as f64).into(),
    )
    .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    js_sys::Reflect::set(
        &obj,
        &"modified_at".into(),
        &(concept.modified_at as f64).into(),
    )
    .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    let expires_at = concept
        .expires_at
        .map_or(JsValue::NULL, |v| (v as f64).into());
    js_sys::Reflect::set(&obj, &"expires_at".into(), &expires_at)
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    let canonical_ids = Array::new();
    for id in &concept.canonical_concept_ids {
        canonical_ids.push(&JsValue::from_str(id));
    }
    js_sys::Reflect::set(&obj, &"canonical_concept_ids".into(), &canonical_ids.into())
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;

    Ok(obj.into())
}

pub(crate) fn to_js_error<E: std::fmt::Display>(error: E) -> JsValue {
    JsValue::from_str(&error.to_string())
}
