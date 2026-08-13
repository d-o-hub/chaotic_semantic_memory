//! MCP Tool and Resource execution logic.

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::handler::McpHandler;
use crate::retrieval::hybrid::HybridResult;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use csm_traits::AbsenceStore;

impl McpHandler {
    pub(crate) async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let fw = self.framework().await?;
        match name {
            "memory_inject" => {
                let id = args["concept_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing concept_id"))?;
                let vector = parse_hvec(&args["vector"])?;
                if let Some(meta) = args.get("metadata") {
                    let meta_map: HashMap<String, Value> = serde_json::from_value(meta.clone())?;
                    fw.inject_concept_with_metadata(id, vector, meta_map)
                        .await?;
                } else {
                    fw.inject_concept(id, vector).await?;
                }
                Ok(json!({"status": "ok", "concept_id": id}))
            }
            "memory_inject_text" => {
                let id = args["concept_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing concept_id"))?;
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;
                if let Some(meta) = args.get("metadata") {
                    let meta_map: HashMap<String, Value> = serde_json::from_value(meta.clone())?;
                    fw.inject_text_with_metadata(id, text, meta_map).await?;
                } else {
                    fw.inject_text(id, text).await?;
                }
                Ok(json!({"status": "ok", "concept_id": id}))
            }
            "memory_probe" => {
                let vector = parse_hvec(&args["vector"])?;
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
                let id = args["concept_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing concept_id"))?;
                if let Some(concept) = fw.get_concept(id).await? {
                    Ok(json!({"status": "ok", "concept": concept}))
                } else {
                    Ok(json!({"status": "not_found"}))
                }
            }
            "memory_delete" => {
                let id = args["concept_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing concept_id"))?;
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
                #[allow(clippy::unwrap_used)] // SystemTime is always after UNIX_EPOCH
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

/// Parse an MCP wire hypervector.
///
/// Primary format (ADR-0094): base64 string of 1280 little-endian bytes
/// (`HVec10240::to_bytes()`).
///
/// Migration fallback: array of 160 JSON integers as u64 halves
/// `(high_0, low_0, high_1, low_1, …)` for each of the 80 `u128` words.
pub(crate) fn parse_hvec(value: &Value) -> Result<csm_core_lib::hyperdim::HVec10240> {
    if let Some(s) = value.as_str() {
        return parse_hvec_base64(s);
    }
    if let Some(arr) = value.as_array() {
        return parse_hvec_u64_halves(arr);
    }
    Err(anyhow::anyhow!(
        "Missing or invalid vector: expected base64 string or array of 160 u64 halves"
    ))
}

fn parse_hvec_base64(s: &str) -> Result<csm_core_lib::hyperdim::HVec10240> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    let bytes = STANDARD
        .decode(s)
        .map_err(|e| anyhow::anyhow!("Invalid base64 vector: {e}"))?;
    csm_core_lib::hyperdim::HVec10240::from_bytes(&bytes)
        .map_err(|e| anyhow::anyhow!("Invalid vector bytes: {e}"))
}

fn parse_hvec_u64_halves(arr: &[Value]) -> Result<csm_core_lib::hyperdim::HVec10240> {
    if arr.len() != 160 {
        return Err(anyhow::anyhow!(
            "Legacy vector must have 160 u64 halves (high, low × 80 words)"
        ));
    }
    let mut data = [0u128; 80];
    for (i, word) in data.iter_mut().enumerate() {
        let high = arr[i * 2]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid vector half at index {}", i * 2))?;
        let low = arr[i * 2 + 1]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid vector half at index {}", i * 2 + 1))?;
        *word = (u128::from(high) << 64) | u128::from(low);
    }
    Ok(csm_core_lib::hyperdim::HVec10240 { data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use csm_core_lib::hyperdim::HVec10240;
    use serde_json::json;

    fn hvec_to_b64(hvec: &HVec10240) -> String {
        STANDARD.encode(hvec.to_bytes())
    }

    fn zero_vector_b64() -> String {
        hvec_to_b64(&HVec10240 { data: [0u128; 80] })
    }

    #[test]
    fn test_parse_hvec_base64_valid() {
        let mut data = [0u128; 80];
        data[0] = 42;
        data[79] = 79;
        let hvec = HVec10240 { data };
        let parsed = parse_hvec(&json!(hvec_to_b64(&hvec))).unwrap();
        assert_eq!(parsed.data[0], 42);
        assert_eq!(parsed.data[79], 79);
    }

    #[test]
    fn test_parse_hvec_base64_high_bits_roundtrip() {
        // JSON integers cannot carry full u128; base64 must preserve bits above 64.
        let mut data = [0u128; 80];
        data[0] = 1u128 << 80;
        data[1] = u128::MAX;
        data[2] = (u64::MAX as u128) << 64 | 0xDEAD_BEEF;
        let original = HVec10240 { data };
        let parsed = parse_hvec(&json!(hvec_to_b64(&original))).unwrap();
        assert_eq!(parsed.data[0], 1u128 << 80);
        assert_eq!(parsed.data[1], u128::MAX);
        assert_eq!(parsed.data[2], (u64::MAX as u128) << 64 | 0xDEAD_BEEF);
        assert_eq!(parsed.to_bytes(), original.to_bytes());
    }

    #[test]
    fn test_parse_hvec_legacy_u64_halves() {
        // 160 halves: (high, low) per word. Word 0 = (1 << 80) = high=1<<16, low=0.
        let mut halves = vec![0u64; 160];
        halves[0] = 1u64 << 16; // high half of word 0
        halves[1] = 0; // low half of word 0
        halves[2] = 0xABCD;
        halves[3] = 0x1234;
        let arr: Vec<Value> = halves.into_iter().map(|n| json!(n)).collect();
        let parsed = parse_hvec(&Value::Array(arr)).unwrap();
        assert_eq!(parsed.data[0], 1u128 << 80);
        assert_eq!(parsed.data[1], (0xABCDu128 << 64) | 0x1234);
    }

    #[test]
    fn test_parse_hvec_invalid_base64() {
        let err = parse_hvec(&json!("not!!!base64")).unwrap_err();
        assert!(err.to_string().contains("Invalid base64"));
    }

    #[test]
    fn test_parse_hvec_wrong_byte_length() {
        let short = STANDARD.encode([0u8; 16]);
        let err = parse_hvec(&json!(short)).unwrap_err();
        assert!(err.to_string().contains("Invalid vector bytes"));
    }

    #[test]
    fn test_parse_hvec_legacy_invalid_length() {
        let arr = vec![json!(1u64); 80];
        let err = parse_hvec(&Value::Array(arr)).unwrap_err();
        assert!(err.to_string().contains("160 u64 halves"));
    }

    #[test]
    fn test_parse_hvec_legacy_invalid_type() {
        let mut arr = vec![json!(1u64); 160];
        arr[0] = json!("not a number");
        let err = parse_hvec(&Value::Array(arr)).unwrap_err();
        assert!(err.to_string().contains("Invalid vector half"));
    }

    #[test]
    fn test_parse_hvec_missing() {
        let err = parse_hvec(&Value::Null).unwrap_err();
        assert!(err.to_string().contains("Missing or invalid vector"));
    }

    /// Helper to create a handler with an in-memory framework.
    async fn handler_with_framework() -> McpHandler {
        let handler = McpHandler::new(None);
        let fw = crate::framework::ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        assert!(handler.framework.set(fw).is_ok());
        handler
    }

    #[tokio::test]
    async fn test_execute_tool_inject_and_get() {
        let handler = handler_with_framework().await;
        let args = json!({
            "concept_id": "test-concept",
            "vector": zero_vector_b64(),
        });
        let result = handler.execute_tool("memory_inject", args).await.unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["concept_id"], "test-concept");

        // Verify get returns the concept
        let get_args = json!({"concept_id": "test-concept"});
        let get_result = handler.execute_tool("memory_get", get_args).await.unwrap();
        assert_eq!(get_result["status"], "ok");
        assert!(get_result["concept"].is_object());
    }

    #[tokio::test]
    async fn test_execute_tool_inject_high_bits_roundtrip() {
        let handler = handler_with_framework().await;
        let mut data = [0u128; 80];
        data[0] = 1u128 << 80;
        data[5] = u128::MAX;
        let original = HVec10240 { data };
        let b64 = hvec_to_b64(&original);

        let inject = json!({
            "concept_id": "high-bits",
            "vector": b64,
        });
        handler.execute_tool("memory_inject", inject).await.unwrap();

        // Probe with the same vector should find the concept (results are (id, score) tuples).
        let probe = json!({
            "vector": hvec_to_b64(&original),
            "top_k": 1,
        });
        let result = handler.execute_tool("memory_probe", probe).await.unwrap();
        assert_eq!(result["status"], "ok");
        let results = result["results"].as_array().unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0][0], "high-bits");

        // Concept serialization uses the same base64 wire format.
        let get = handler
            .execute_tool("memory_get", json!({"concept_id": "high-bits"}))
            .await
            .unwrap();
        let wire = get["concept"]["vector"]
            .as_str()
            .expect("concept.vector must be base64 string");
        let restored = parse_hvec(&json!(wire)).unwrap();
        assert_eq!(restored.data[0], 1u128 << 80);
        assert_eq!(restored.data[5], u128::MAX);
        assert_eq!(restored.to_bytes(), original.to_bytes());
    }

    #[tokio::test]
    async fn test_execute_tool_inject_text() {
        let handler = handler_with_framework().await;
        let args = json!({
            "concept_id": "text-concept",
            "text": "hello world",
        });
        let result = handler
            .execute_tool("memory_inject_text", args)
            .await
            .unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["concept_id"], "text-concept");
    }

    #[tokio::test]
    async fn test_execute_tool_delete() {
        let handler = handler_with_framework().await;
        // Inject first
        let inject_args = json!({
            "concept_id": "to-delete",
            "vector": zero_vector_b64(),
        });
        handler
            .execute_tool("memory_inject", inject_args)
            .await
            .unwrap();

        // Delete
        let del_args = json!({"concept_id": "to-delete"});
        let result = handler
            .execute_tool("memory_delete", del_args)
            .await
            .unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["deleted"], true);

        // Verify it's gone
        let get_args = json!({"concept_id": "to-delete"});
        let get_result = handler.execute_tool("memory_get", get_args).await.unwrap();
        assert_eq!(get_result["status"], "not_found");
    }

    #[tokio::test]
    async fn test_execute_tool_get_not_found() {
        let handler = handler_with_framework().await;
        let args = json!({"concept_id": "nonexistent"});
        let result = handler.execute_tool("memory_get", args).await.unwrap();
        assert_eq!(result["status"], "not_found");
    }

    #[tokio::test]
    async fn test_execute_tool_missing_concept_id() {
        let handler = handler_with_framework().await;
        let args = json!({"wrong_field": "value"});
        let err = handler
            .execute_tool("memory_inject_text", args)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Missing concept_id"));
    }
}
