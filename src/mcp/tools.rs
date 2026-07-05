//! MCP tool and resource execution logic (ADR-0067)
//!
//! Implements `execute_tool` / `execute_read_resource` dispatch plus
//! BM25 indexing and hypervector parsing helpers for [`McpHandler`].

use std::collections::HashMap;
#[cfg(test)]
use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};

use super::handler::McpHandler;
use crate::retrieval::{
    HybridConfig, HybridMode, HybridResult, compute_weights, merge_results_checked,
};

impl McpHandler {
    pub(super) async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
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
                let results = fw.probe(vector, top_k).await?;
                Ok(json!({"status": "ok", "results": results}))
            }
            "memory_probe_text" => {
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let min_score = args["min_score"].as_f64().unwrap_or(0.0) as f32;

                // 1. Semantic retrieval (HDC)
                let hdc_results = fw.probe_text(text, top_k).await?;

                // 2. Keyword retrieval (BM25)
                let bm25_index = build_bm25_index(fw).await?;
                let query_tokens = tokenize_query(text);
                let bm25_results = if bm25_index.is_empty() {
                    Vec::new()
                } else {
                    bm25_index.search(&query_tokens, top_k)
                };

                // 3. Hybrid merge
                let weights = compute_weights(query_tokens.len());
                let config = HybridConfig {
                    mode: HybridMode::Auto,
                    min_score,
                };

                match merge_results_checked(&bm25_results, &hdc_results, weights, &config, text) {
                    HybridResult::Hits(results) => Ok(json!({
                        "status": "ok",
                        "results": results
                    })),
                    HybridResult::Abstained(abstention) => Ok(json!({
                        "status": "abstained",
                        "abstained": true,
                        "reason": "No concepts match query above confidence threshold",
                        "query": abstention.query,
                        "best_score_seen": abstention.best_score_seen,
                        "threshold": abstention.min_score_threshold,
                        "timestamp": abstention.timestamp,
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

                let results = fw.probe_text_filtered(text, top_k, &filter).await?;
                Ok(json!({"status": "ok", "results": results}))
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
            _ => Err(anyhow::anyhow!("Tool not implemented: {name}")),
        }
    }

    pub(super) async fn execute_read_resource(&self, uri: &str) -> Result<Value> {
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

fn parse_hvec(vec_data: &[Value]) -> Result<csm_core::hyperdim::HVec10240> {
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

/// Tokenize query text for BM25.
fn tokenize_query(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Build BM25 index from concepts in the framework.
async fn build_bm25_index(
    framework: &crate::framework::ChaoticSemanticFramework,
) -> anyhow::Result<crate::retrieval::bm25::Bm25Index> {
    let concepts = {
        let singularity = framework.singularity();
        let sing = singularity.read().await;
        let ns = framework.namespace().await;
        sing.all_concepts(&ns)
    };

    let mut index = crate::retrieval::bm25::Bm25Index::new();
    for concept in concepts {
        let text = concept
            .metadata
            .get("text_preview")
            .or_else(|| concept.metadata.get("content_preview"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let tokens: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if !tokens.is_empty() {
            index.add_document(&concept.id, &tokens);
        }
    }

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_hvec_roundtrips_values() {
        let input: Vec<Value> = (0u64..80).map(|i| json!(i * 3 + 7)).collect();
        let hvec = parse_hvec(&input).expect("must parse successfully");
        for (i, val) in hvec.data.iter().enumerate() {
            assert_eq!(*val, (i as u128) * 3 + 7, "element {i} mismatch");
        }
    }

    #[test]
    fn parse_hvec_rejects_wrong_length() {
        let short: Vec<Value> = vec![json!(1u64); 10];
        assert!(
            parse_hvec(&short).is_err(),
            "must reject non-80-element input"
        );
    }

    #[test]
    fn parse_hvec_known_values_match_expected() {
        let input: Vec<Value> = vec![json!(0u64), json!(1), json!(255), json!(u64::MAX)]
            .into_iter()
            .chain((4..80).map(|i| json!(i as u64)))
            .collect();
        let hvec = parse_hvec(&input).expect("must parse");
        assert_eq!(hvec.data[0], 0u128, "first element must be 0");
        assert_eq!(hvec.data[1], 1u128, "second element must be 1");
        assert_eq!(hvec.data[2], 255u128, "third element must be 255");
        assert_eq!(
            hvec.data[3],
            u64::MAX as u128,
            "fourth element must be u64::MAX"
        );
        assert_eq!(hvec.data[79], 79u128, "last element must be 79");
    }

    #[test]
    fn parse_hvec_all_zeros_roundtrips() {
        let input: Vec<Value> = (0..80).map(|_| json!(0u64)).collect();
        let hvec = parse_hvec(&input).expect("must parse zeros");
        assert!(
            hvec.data.iter().all(|&v| v == 0),
            "all elements must be zero"
        );
    }

    #[test]
    fn parse_hvec_rejects_non_numeric() {
        let mut input: Vec<Value> = (0..80).map(|i| json!(i as u64)).collect();
        input[40] = json!("not a number");
        assert!(
            parse_hvec(&input).is_err(),
            "must reject non-numeric element"
        );
    }

    #[test]
    fn test_mcp_handler_clone_initializes_separate_framework() {
        let db_path = Some(PathBuf::from("test_mcp_clone.db"));
        let handler1 = McpHandler::new(db_path.clone());
        let handler2 = handler1.clone();

        assert!(!handler1.is_framework_initialized());
        assert!(!handler2.is_framework_initialized());

        assert_eq!(handler1.database, db_path);
        assert_eq!(handler2.database, db_path);
    }
}
