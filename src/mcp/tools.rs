//! MCP Tool and Resource execution logic.

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
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
                let id = args["concept_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing concept_id"))?;
                let vector = args["vector"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing vector (expected base64 string)"))?;
                let vector = parse_hvec(vector)?;
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
                let vector = args["vector"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing vector (expected base64 string)"))?;
                let vector = parse_hvec(vector)?;
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

/// Parse a canonical base64-encoded hypervector from the MCP wire format.
///
/// The wire format is STANDARD base64 of [`HVec10240::to_bytes`]: the 80
/// u128 words serialized as 16 little-endian bytes each, 1280 bytes total.
/// Invalid base64 or any decoded length other than 1280 bytes is rejected.
pub(crate) fn parse_hvec(encoded: &str) -> Result<csm_core::hyperdim::HVec10240> {
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| anyhow::anyhow!("Vector must be valid base64: {e}"))?;
    csm_core::hyperdim::HVec10240::from_bytes(&bytes)
        .map_err(|e| anyhow::anyhow!("Invalid vector: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_hvec_roundtrip_high_bits() {
        // Every u128 word set to 0xA5A5... exercises all 128 bits and both
        // byte orders: little-endian serialization must round-trip exactly.
        let mut hvec = csm_core::hyperdim::HVec10240::zero();
        for word in &mut hvec.data {
            *word = 0xA5A5_A5A5_A5A5_A5A5_A5A5_A5A5_A5A5_A5A5u128;
        }
        let encoded = STANDARD.encode(hvec.to_bytes());
        assert_eq!(encoded.len(), 1708); // 1280 bytes -> STANDARD base64
        let parsed = parse_hvec(&encoded).unwrap();
        assert_eq!(parsed, hvec);
        assert_eq!(parsed.to_bytes(), hvec.to_bytes());
    }

    #[test]
    fn test_parse_hvec_rejects_wrong_length() {
        // Valid base64 of 100 bytes: decodes fine but is not 1280 bytes.
        let encoded = STANDARD.encode(vec![0xABu8; 100]);
        let err = parse_hvec(&encoded).unwrap_err();
        assert!(
            err.to_string().contains("1280"),
            "expected length rejection, got: {err}"
        );
    }

    #[test]
    fn test_parse_hvec_rejects_invalid_base64() {
        let err = parse_hvec("!!!not-base64!!!").unwrap_err();
        assert!(
            err.to_string().contains("base64"),
            "expected base64 rejection, got: {err}"
        );
    }

    /// A hypervector whose 80 u128 words are all 0xA5A5..., base64-encoded.
    fn high_bits_vector_encoded() -> String {
        let mut hvec = csm_core::hyperdim::HVec10240::zero();
        for word in &mut hvec.data {
            *word = 0xA5A5_A5A5_A5A5_A5A5_A5A5_A5A5_A5A5_A5A5u128;
        }
        STANDARD.encode(hvec.to_bytes())
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
        let encoded = high_bits_vector_encoded();
        let args = json!({
            "concept_id": "test-concept",
            "vector": encoded,
        });
        let result = handler.execute_tool("memory_inject", args).await.unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["concept_id"], "test-concept");

        // Verify get returns the concept with a base64 vector that
        // round-trips to the injected high-bit words.
        let get_args = json!({"concept_id": "test-concept"});
        let get_result = handler.execute_tool("memory_get", get_args).await.unwrap();
        assert_eq!(get_result["status"], "ok");
        let vector_str = get_result["concept"]["vector"].as_str().unwrap();
        let parsed = parse_hvec(vector_str).unwrap();
        let mut expected = csm_core::hyperdim::HVec10240::zero();
        for word in &mut expected.data {
            *word = 0xA5A5_A5A5_A5A5_A5A5_A5A5_A5A5_A5A5_A5A5u128;
        }
        assert_eq!(parsed, expected);
    }

    #[tokio::test]
    async fn test_execute_tool_probe_base64_vector() {
        let handler = handler_with_framework().await;
        let query_encoded = high_bits_vector_encoded();
        let other_encoded = STANDARD.encode(vec![0x5Au8; 1280]);

        handler
            .execute_tool(
                "memory_inject",
                json!({"concept_id": "p1", "vector": query_encoded}),
            )
            .await
            .unwrap();
        handler
            .execute_tool(
                "memory_inject",
                json!({"concept_id": "p2", "vector": other_encoded}),
            )
            .await
            .unwrap();

        let probe_args = json!({"vector": query_encoded, "top_k": 5});
        let result = handler
            .execute_tool("memory_probe", probe_args)
            .await
            .unwrap();
        assert_eq!(result["status"], "ok");
        let results = result["results"].as_array().unwrap();
        assert!(
            !results.is_empty(),
            "probe should return at least one result"
        );
        // Exact match must rank first: results are [concept_id, score] tuples.
        assert_eq!(results[0][0], "p1");
    }

    #[tokio::test]
    async fn test_execute_tool_rejects_integer_array_vector() {
        // Wave 32 P1: the legacy JSON integer-word array is replaced by the
        // canonical base64 string and must be rejected at the handler level.
        let handler = handler_with_framework().await;
        let legacy: Vec<Value> = (0..80).map(|i| json!(i as u64)).collect();
        let args = json!({"concept_id": "old-style", "vector": legacy});
        let err = handler
            .execute_tool("memory_inject", args)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("base64"),
            "expected base64 string rejection, got: {err}"
        );
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
            "vector": high_bits_vector_encoded(),
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
