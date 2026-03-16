use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Payload for JSON export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExportPayload {
    pub(crate) version: String,
    pub(crate) exported_at: u64,
    pub(crate) concepts: Vec<crate::singularity::Concept>,
    pub(crate) associations: Vec<(String, String, f32)>,
}

/// Binary-compatible metadata value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum BinaryMetadataValue {
    Null,
    Bool(bool),
    Number(String), // Store numbers as strings to preserve precision
    String(String),
    Array(Vec<BinaryMetadataValue>),
    Object(HashMap<String, BinaryMetadataValue>),
}

impl From<serde_json::Value> for BinaryMetadataValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => BinaryMetadataValue::Null,
            serde_json::Value::Bool(b) => BinaryMetadataValue::Bool(b),
            serde_json::Value::Number(n) => BinaryMetadataValue::Number(n.to_string()),
            serde_json::Value::String(s) => BinaryMetadataValue::String(s),
            serde_json::Value::Array(arr) => {
                BinaryMetadataValue::Array(arr.into_iter().map(BinaryMetadataValue::from).collect())
            }
            serde_json::Value::Object(obj) => BinaryMetadataValue::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, BinaryMetadataValue::from(v)))
                    .collect(),
            ),
        }
    }
}

impl From<BinaryMetadataValue> for serde_json::Value {
    fn from(value: BinaryMetadataValue) -> Self {
        match value {
            BinaryMetadataValue::Null => serde_json::Value::Null,
            BinaryMetadataValue::Bool(b) => serde_json::Value::Bool(b),
            BinaryMetadataValue::Number(n) => {
                serde_json::Value::Number(n.parse().unwrap_or(serde_json::Number::from(0)))
            }
            BinaryMetadataValue::String(s) => serde_json::Value::String(s),
            BinaryMetadataValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            BinaryMetadataValue::Object(obj) => serde_json::Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect(),
            ),
        }
    }
}

/// Concept representation for binary export (bincode-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BinaryConcept {
    pub(crate) id: String,
    /// Raw bytes of the HVec10240 (1280 bytes)
    pub(crate) vector_bytes: Vec<u8>,
    pub(crate) metadata: HashMap<String, BinaryMetadataValue>,
    pub(crate) created_at: u64,
    pub(crate) modified_at: u64,
}

impl From<crate::singularity::Concept> for BinaryConcept {
    fn from(concept: crate::singularity::Concept) -> Self {
        Self {
            id: concept.id,
            vector_bytes: concept.vector.to_bytes(),
            metadata: concept
                .metadata
                .into_iter()
                .map(|(k, v)| (k, BinaryMetadataValue::from(v)))
                .collect(),
            created_at: concept.created_at,
            modified_at: concept.modified_at,
        }
    }
}

impl BinaryConcept {
    pub(crate) fn to_concept(&self) -> crate::error::Result<crate::singularity::Concept> {
        Ok(crate::singularity::Concept {
            id: self.id.clone(),
            vector: crate::hyperdim::HVec10240::from_bytes(&self.vector_bytes)?,
            metadata: self
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::from(v.clone())))
                .collect(),
            created_at: self.created_at,
            modified_at: self.modified_at,
        })
    }
}

/// Payload for binary export/import (bincode-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BinaryExportPayload {
    pub(crate) version: String,
    pub(crate) exported_at: u64,
    pub(crate) concepts: Vec<BinaryConcept>,
    pub(crate) associations: Vec<(String, String, f32)>,
}

impl From<ExportPayload> for BinaryExportPayload {
    fn from(payload: ExportPayload) -> Self {
        Self {
            version: payload.version,
            exported_at: payload.exported_at,
            concepts: payload
                .concepts
                .into_iter()
                .map(BinaryConcept::from)
                .collect(),
            associations: payload.associations,
        }
    }
}

impl BinaryExportPayload {
    pub(crate) fn to_export_payload(&self) -> crate::error::Result<ExportPayload> {
        let mut concepts = Vec::with_capacity(self.concepts.len());
        for bc in &self.concepts {
            concepts.push(bc.to_concept()?);
        }
        Ok(ExportPayload {
            version: self.version.clone(),
            exported_at: self.exported_at,
            concepts,
            associations: self.associations.clone(),
        })
    }
}

pub(crate) fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
