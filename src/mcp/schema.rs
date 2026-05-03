use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input for memory_inject tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryInjectInput {
    pub id: String,
    pub vector: Vec<u8>, // 1280 bytes for 10240 bits
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Input for memory_inject_text tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryInjectTextInput {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Input for memory_probe tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryProbeInput {
    pub vector: Vec<u8>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

/// Input for memory_probe_text tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryProbeTextInput {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

/// Input for memory_probe_filtered tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryProbeFilteredInput {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    pub filter: crate::metadata_filter::MetadataFilter,
}

/// Input for memory_get tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryGetInput {
    pub id: String,
}

/// Input for memory_delete tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryDeleteInput {
    pub id: String,
}

/// Input for memory_associate tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryAssociateInput {
    pub from: String,
    pub to: String,
    #[serde(default = "default_strength")]
    pub strength: f32,
}

/// Input for memory_traverse tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryTraverseInput {
    pub start_id: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default)]
    pub min_strength: f32,
}

/// Input for memory_shortest_path tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryShortestPathInput {
    pub from: String,
    pub to: String,
}

/// Input for memory_stats tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryStatsInput {}

/// Input for memory_export tool.
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct MemoryExportInput {
    #[serde(default = "default_export_path")]
    pub path: String,
}

fn default_top_k() -> usize {
    10
}
fn default_strength() -> f32 {
    1.0
}
fn default_max_depth() -> u32 {
    3
}
fn default_export_path() -> String {
    "export.json".to_string()
}
