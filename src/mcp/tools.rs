//! MCP Tool and Resource execution logic.

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::handler::McpHandler;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::bridge_persistence::AbsenceStore;
use crate::retrieval::hybrid::HybridResult;

impl McpHandler {
    pub(crate) async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let fw = self.framework().await?;
        match name {
            "memory_inject" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                let vec_data = args["vector"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing vector"))?;
                let vector = parse_hvec(vec_data)?;
                if let Some(meta) = args.get("metadata") {
                    let meta_map: HashMap<String, Value> = serde_json::from_value(meta.clone())?;
                    fw.inject_concept_with_metadata(id, vector, meta_map)
                        .await?;
                } else {
                    fw.inject_concept(id, vector).await?;
                }
                Ok(json!({"status": "ok", "id": id}))
            }
            "memory_inject_text" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;
                if let Some(meta) = args.get("metadata") {
                    let meta_map: HashMap<String, Value> = serde_json::from_value(meta.clone())?;
                    fw.inject_text_with_metadata(id, text, meta_map).await?;
                } else {
                    fw.inject_text(id, text).await?;
                }
                Ok(json!({"status": "ok", "id": id}))
            }
            "memory_probe" => {
                let vec_data = args["vector"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing vector"))?;
                let vector = parse_hvec(vec_data)?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let (results, _) = fw.probe_with_best_score(vector, top_k).await?;
                Ok(json!({"status": "ok", "results": results}))
            }
            "memory_probe_text" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query"))?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let result = fw.probe_text(query, top_k).await?;
                match result {
                    HybridResult::Success(results) => {
                        Ok(json!({"status": "ok", "results": results}))
                    }
                    HybridResult::Abstained(abstention) => Ok(json!({
                        "status": "abstained",
                        "reason": "No concepts match query above confidence threshold",
                        "query": abstention.query,
                        "best_score_seen": abstention.best_score_seen,
                        "threshold": abstention.min_score_threshold,
                    })),
                }
            }
            "memory_probe_filtered" => {
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let filter_val = args
                    .get("filter")
                    .ok_or_else(|| anyhow::anyhow!("Missing filter"))?;
                let filter: crate::metadata_filter::MetadataFilter =
                    serde_json::from_value(filter_val.clone())?;

                let result = fw.probe_text_filtered(text, top_k, &filter).await?;
                match result {
                    HybridResult::Success(results) => {
                        Ok(json!({"status": "ok", "results": results}))
                    }
                    HybridResult::Abstained(abstention) => Ok(json!({
                        "status": "abstained",
                        "reason": "No concepts match query above confidence threshold",
                        "query": abstention.query,
                        "best_score_seen": abstention.best_score_seen,
                        "threshold": abstention.min_score_threshold,
                    })),
                }
            }
            "memory_get" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                if let Some(concept) = fw.get_concept(id).await? {
                    Ok(json!({"status": "ok", "concept": concept}))
                } else {
                    Ok(json!({"status": "not_found"}))
                }
            }
            "memory_delete" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                fw.delete_concept(id).await?;
                Ok(json!({"status": "ok", "deleted": true}))
            }
            "memory_associate" => {
                let from = args["from_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing from_id"))?;
                let to = args["to_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing to_id"))?;
                let strength = args["strength"].as_f64().unwrap_or(0.5) as f32;
                fw.associate(from, to, strength).await?;
                Ok(json!({"status": "ok"}))
            }
            "memory_traverse" => {
                let start_id = args["start_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing start_id"))?;
                let depth = args["depth"].as_u64().unwrap_or(3) as usize;
                let min_strength = args["min_strength"].as_f64().unwrap_or(0.0) as f32;
                let config = crate::graph_traversal::TraversalConfig {
                    max_depth: depth,
                    min_strength,
                    max_results: 100,
                };
                let results = fw.traverse(start_id, config).await?;
                Ok(json!({"status": "ok", "nodes": results}))
            }
            "memory_shortest_path" => {
                let from = args["from_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing from_id"))?;
                let to = args["to_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing to_id"))?;
                let path = fw.shortest_path(from, to).await?;
                Ok(json!({"status": "ok", "path": path}))
            }
            "memory_stats" => {
                let stats = fw.stats().await?;
                Ok(json!({"status": "ok", "stats": stats}))
            }
            "memory_export" => {
                let format = args["format"].as_str().unwrap_or("json");
                let path = format!(
                    "export_{}.{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    format
                );
                if format == "binary" {
                    fw.export_binary(&path).await?;
                } else {
                    fw.export_json(&path).await?;
                }
                Ok(json!({"status": "ok", "file": path}))
            }
            "memory_list_gaps" => {
                let min_attempts = args["min_attempts"].as_u64().unwrap_or(1) as u32;
                #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
                if let Some(ref store) = fw.persistence {
                    let entries = store.list_absences(min_attempts).await?;
                    let total = entries.len();
                    Ok(json!({
                        "status": "ok",
                        "gaps": entries,
                        "total": total,
                    }))
                } else {
                    Err(anyhow::anyhow!("Persistence not enabled"))
                }
                #[cfg(any(target_arch = "wasm32", not(feature = "persistence")))]
                {
                    let _ = min_attempts;
                    Err(anyhow::anyhow!("Persistence not enabled"))
                }
            }
            _ => Err(anyhow::anyhow!("Tool not implemented: {name}")),
        }
    }

    pub(crate) async fn execute_read_resource(&self, uri: &str) -> Result<Value> {
        let fw = self.framework().await?;
        if let Some(id) = uri.strip_prefix("concept://") {
            if let Some(concept) = fw.get_concept(id).await? {
                Ok(json!(concept))
            } else {
                Err(anyhow::anyhow!("Concept not found: {id}"))
            }
        } else if uri == "stats://current" {
            let stats = fw.stats().await?;
            Ok(json!(stats))
        } else if uri == "health://current" {
            fw.persistence_health_check().await?;
            Ok(json!({"status": "healthy"}))
        } else {
            Err(anyhow::anyhow!("Unknown resource URI: {uri}"))
        }
    }
}

pub(crate) fn parse_hvec(vec_data: &[Value]) -> Result<csm_core::hyperdim::HVec10240> {
    if vec_data.len() != 80 {
        return Err(anyhow::anyhow!("Vector must have 80 elements"));
    }
    let mut data = [0u128; 80];
    for (i, val) in vec_data.iter().enumerate() {
        data[i] = val
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid vector element"))? as u128;
    }
    Ok(csm_core::hyperdim::HVec10240 { data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_hvec_valid() {
        let vec_data: Vec<Value> = (0..80).map(|i| json!(i as u64)).collect();
        let hvec = parse_hvec(&vec_data).unwrap();
        assert_eq!(hvec.data[0], 0);
        assert_eq!(hvec.data[79], 79);
    }

    #[test]
    fn test_parse_hvec_invalid_length() {
        let vec_data = vec![json!(1u64); 79];
        let err = parse_hvec(&vec_data).unwrap_err();
        assert_eq!(err.to_string(), "Vector must have 80 elements");
    }

    #[test]
    fn test_parse_hvec_invalid_type() {
        let mut vec_data = vec![json!(1u64); 80];
        vec_data[0] = json!("not a number");
        let err = parse_hvec(&vec_data).unwrap_err();
        assert_eq!(err.to_string(), "Invalid vector element");
    }
}
