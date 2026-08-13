//! Shared types and traits for chaotic_semantic_memory workspace.
//!
//! This crate provides types that are needed by multiple workspace crates
//! but aren't yet in csm-core or csm-memory.

use csm_core_lib::hyperdim::HVec10240;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod absence;

pub use absence::{AbsenceEntry, AbsenceStore};

// ============================================================================
// Constants
// ============================================================================

/// Maximum import size in bytes (100MB).
pub const MAX_IMPORT_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum filter depth to prevent stack overflow.
pub const MAX_FILTER_DEPTH: usize = 32;

// ============================================================================
// Memory Events
// ============================================================================

/// Memory events emitted by the framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryEvent {
    /// A concept was injected.
    ConceptInjected { id: String, timestamp: u64 },
    /// A concept was updated.
    ConceptUpdated { id: String, timestamp: u64 },
    /// A concept was deleted.
    ConceptDeleted { id: String, timestamp: u64 },
    /// An association was created.
    Associated {
        from: String,
        to: String,
        strength: f32,
    },
    /// An association was removed.
    Disassociated { from: String, to: String },
}

// ============================================================================
// Export/Import Types
// ============================================================================

/// JSON-based export payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
    pub version: String,
    pub exported_at: u64,
    pub concepts: Vec<ExportConcept>,
    pub associations: Vec<(String, String, f32)>,
}

/// Concept in export payload (uses serde_json::Value for metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConcept {
    pub id: String,
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    pub expires_at: Option<u64>,
    pub canonical_concept_ids: Vec<String>,
}

/// Binary-optimized export payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryExportPayload {
    pub version: String,
    pub exported_at: u64,
    pub concepts: Vec<BinaryConcept>,
    pub associations: Vec<(String, String, f32)>,
}

/// Binary-optimized concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryConcept {
    pub id: String,
    pub vector_bytes: Vec<u8>,
    pub metadata: HashMap<String, BinaryMetadataValue>,
    pub created_at: u64,
    pub modified_at: u64,
    pub expires_at: Option<u64>,
    pub canonical_concept_ids: Vec<String>,
}

/// Binary-optimized metadata value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryMetadataValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<BinaryMetadataValue>),
    Object(HashMap<String, BinaryMetadataValue>),
}

impl From<serde_json::Value> for BinaryMetadataValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => BinaryMetadataValue::Null,
            serde_json::Value::Bool(b) => BinaryMetadataValue::Bool(b),
            serde_json::Value::Number(n) => BinaryMetadataValue::Number(n.to_string()),
            serde_json::Value::String(s) => BinaryMetadataValue::String(s),
            serde_json::Value::Array(arr) => {
                BinaryMetadataValue::Array(arr.into_iter().map(Into::into).collect())
            }
            serde_json::Value::Object(obj) => {
                BinaryMetadataValue::Object(obj.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl From<BinaryMetadataValue> for serde_json::Value {
    fn from(b: BinaryMetadataValue) -> Self {
        match b {
            BinaryMetadataValue::Null => serde_json::Value::Null,
            BinaryMetadataValue::Bool(b) => serde_json::Value::Bool(b),
            BinaryMetadataValue::Number(s) => {
                serde_json::Value::Number(s.parse().unwrap_or(0.into()))
            }
            BinaryMetadataValue::String(s) => serde_json::Value::String(s),
            BinaryMetadataValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Into::into).collect())
            }
            BinaryMetadataValue::Object(obj) => {
                serde_json::Value::Object(obj.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl From<ExportPayload> for BinaryExportPayload {
    fn from(p: ExportPayload) -> Self {
        BinaryExportPayload {
            version: p.version,
            exported_at: p.exported_at,
            concepts: p.concepts.into_iter().map(Into::into).collect(),
            associations: p.associations,
        }
    }
}

impl From<ExportConcept> for BinaryConcept {
    fn from(c: ExportConcept) -> Self {
        BinaryConcept {
            id: c.id,
            vector_bytes: c.vector.to_bytes().to_vec(),
            metadata: c.metadata.into_iter().map(|(k, v)| (k, v.into())).collect(),
            created_at: c.created_at,
            modified_at: c.modified_at,
            expires_at: c.expires_at,
            canonical_concept_ids: c.canonical_concept_ids,
        }
    }
}

impl BinaryConcept {
    /// Convert back to ExportConcept.
    pub fn to_export_concept(&self) -> csm_core_lib::error::Result<ExportConcept> {
        let vector = HVec10240::from_bytes(&self.vector_bytes)?;
        let metadata = self
            .metadata
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        Ok(ExportConcept {
            id: self.id.clone(),
            vector,
            metadata,
            created_at: self.created_at,
            modified_at: self.modified_at,
            expires_at: self.expires_at,
            canonical_concept_ids: self.canonical_concept_ids.clone(),
        })
    }
}

impl BinaryExportPayload {
    /// Convert back to ExportPayload.
    pub fn to_export_payload(&self) -> csm_core_lib::error::Result<ExportPayload> {
        Ok(ExportPayload {
            version: self.version.clone(),
            exported_at: self.exported_at,
            concepts: self
                .concepts
                .iter()
                .map(|c| c.to_export_concept())
                .collect::<Result<Vec<_>, _>>()?,
            associations: self.associations.clone(),
        })
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get current time in Unix seconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Get current time in Unix seconds (WASM version).
#[cfg(target_arch = "wasm32")]
pub fn unix_now_secs() -> u64 {
    (js_sys::Date::new_0().get_time() / 1000.0) as u64
}

/// Get current time in Unix nanoseconds.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::cast_possible_truncation)] // .min(u64::MAX) ensures value fits
pub fn unix_now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(u128::from(u64::MAX)) as u64
}

/// Get current time in Unix nanoseconds (WASM version).
#[cfg(target_arch = "wasm32")]
pub fn unix_now_ns() -> u64 {
    (js_sys::Date::new_0().get_time() * 1_000_000.0) as u64
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_unix_now_secs() {
        let t = unix_now_secs();
        assert!(t > 0);
    }

    #[test]
    fn test_unix_now_ns() {
        let t = unix_now_ns();
        assert!(t > 0);
    }

    #[test]
    fn test_binary_metadata_roundtrip() {
        let values = vec![
            BinaryMetadataValue::Null,
            BinaryMetadataValue::Bool(true),
            BinaryMetadataValue::Number("42".to_string()),
            BinaryMetadataValue::String("hello".to_string()),
        ];
        for v in values {
            let json: serde_json::Value = v.clone().into();
            let back: BinaryMetadataValue = json.clone().into();
            let json2: serde_json::Value = back.into();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_export_payload_roundtrip() {
        let payload = ExportPayload {
            version: "1.0".to_string(),
            exported_at: 1234567890,
            concepts: vec![],
            associations: vec![],
        };
        let binary: BinaryExportPayload = payload.into();
        let back = binary.to_export_payload().unwrap();
        assert_eq!(back.version, "1.0");
        assert_eq!(back.exported_at, 1234567890);
    }
}
