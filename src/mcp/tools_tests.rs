//! Tests for MCP tool and resource execution logic.

use super::*;
use crate::metadata_filter::MetadataFilter;
use serde_json::json;

#[test]
fn test_parse_hvec_valid() {
    let vec_data: Vec<Value> = (0..80).map(|i| json!(i as u64)).collect();
    let hvec = parse_hvec(&vec_data).unwrap();
    assert_eq!(hvec.data[0], 0);
    assert_eq!(hvec.data[79], 79);
}

#[test]
fn test_parse_hvec_invalid_length_short() {
    let vec_data = vec![json!(1u64); 79];
    let err = parse_hvec(&vec_data).unwrap_err();
    assert_eq!(err.to_string(), "Vector must have 80 elements");
}

#[test]
fn test_parse_hvec_invalid_length_long() {
    let vec_data = vec![json!(0u64); 81];
    let err = parse_hvec(&vec_data).unwrap_err();
    assert_eq!(err.to_string(), "Vector must have 80 elements");
}

#[test]
fn test_parse_hvec_empty() {
    let vec_data: Vec<Value> = vec![];
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

#[test]
fn test_parse_hvec_max_values() {
    let vec_data: Vec<Value> = (0..80).map(|_| json!(u64::MAX)).collect();
    let hvec = parse_hvec(&vec_data).unwrap();
    assert_eq!(hvec.data[0], u64::MAX as u128);
}

#[test]
fn test_parse_hvec_all_zeros() {
    let vec_data: Vec<Value> = (0..80).map(|_| json!(0u64)).collect();
    let hvec = parse_hvec(&vec_data).unwrap();
    for val in &hvec.data {
        assert_eq!(*val, 0);
    }
}

// --- execute_tool tests ---

#[tokio::test]
async fn test_execute_tool_unknown_tool() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_tool("unknown_tool", json!({}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Tool not implemented"));
}

#[tokio::test]
async fn test_execute_tool_memory_inject() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|i| json!(i as u64)).collect();
    let result = handler
        .execute_tool("memory_inject", json!({"id": "t1", "vector": vec_data}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], "t1");
}

#[tokio::test]
async fn test_execute_tool_memory_inject_missing_id() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(0u64)).collect();
    let err = handler
        .execute_tool("memory_inject", json!({"vector": vec_data}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Missing id"));
}

#[tokio::test]
async fn test_execute_tool_memory_inject_missing_vector() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_tool("memory_inject", json!({"id": "t1"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Missing vector"));
}

#[tokio::test]
async fn test_execute_tool_memory_inject_text() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool(
            "memory_inject_text",
            json!({"id": "t2", "text": "hello world"}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], "t2");
}

#[tokio::test]
async fn test_execute_tool_memory_inject_text_missing_text() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_tool("memory_inject_text", json!({"id": "t2"}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Missing text"));
}

#[tokio::test]
async fn test_execute_tool_memory_inject_with_metadata() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(1u64)).collect();
    let result = handler
        .execute_tool(
            "memory_inject",
            json!({"id": "meta1", "vector": vec_data, "metadata": {"type": "test"}}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
}

#[tokio::test]
async fn test_execute_tool_memory_inject_text_with_metadata() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool(
            "memory_inject_text",
            json!({"id": "meta2", "text": "hello", "metadata": {"type": "test"}}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
}

#[tokio::test]
async fn test_execute_tool_memory_probe() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(0u64)).collect();
    let result = handler
        .execute_tool("memory_probe", json!({"vector": vec_data, "top_k": 3}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert!(result["results"].is_array());
}

#[tokio::test]
async fn test_execute_tool_memory_probe_text() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool(
            "memory_probe_text",
            json!({"query": "test query", "top_k": 5}),
        )
        .await
        .unwrap();
    assert!(
        result["status"] == "ok" || result["status"] == "abstained",
        "Expected ok or abstained, got: {result}"
    );
}

#[tokio::test]
async fn test_execute_tool_memory_probe_filtered() {
    let handler = McpHandler::new(None);
    let filter = MetadataFilter::eq("type", "memory");
    let filter_json = serde_json::to_value(&filter).unwrap();
    let result = handler
        .execute_tool(
            "memory_probe_filtered",
            json!({"text": "test", "top_k": 5, "filter": filter_json}),
        )
        .await
        .unwrap();
    assert!(
        result["status"] == "ok" || result["status"] == "abstained",
        "Expected ok or abstained, got: {result}"
    );
}

#[tokio::test]
async fn test_execute_tool_memory_probe_filtered_missing_filter() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_tool("memory_probe_filtered", json!({"text": "test", "top_k": 5}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Missing filter"));
}

#[tokio::test]
async fn test_execute_tool_memory_get_not_found() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool("memory_get", json!({"id": "nonexistent"}))
        .await
        .unwrap();
    assert_eq!(result["status"], "not_found");
}

#[tokio::test]
async fn test_execute_tool_memory_get_after_inject() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(1u64)).collect();
    handler
        .execute_tool("memory_inject", json!({"id": "g1", "vector": vec_data}))
        .await
        .unwrap();
    let result = handler
        .execute_tool("memory_get", json!({"id": "g1"}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert!(result["concept"].is_object());
}

#[tokio::test]
async fn test_execute_tool_memory_delete() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(1u64)).collect();
    handler
        .execute_tool("memory_inject", json!({"id": "d1", "vector": vec_data}))
        .await
        .unwrap();
    let result = handler
        .execute_tool("memory_delete", json!({"id": "d1"}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert_eq!(result["deleted"], true);
}

#[tokio::test]
async fn test_execute_tool_memory_associate() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(1u64)).collect();
    handler
        .execute_tool(
            "memory_inject",
            json!({"id": "a1", "vector": vec_data.clone()}),
        )
        .await
        .unwrap();
    handler
        .execute_tool("memory_inject", json!({"id": "a2", "vector": vec_data}))
        .await
        .unwrap();
    let result = handler
        .execute_tool(
            "memory_associate",
            json!({"from_id": "a1", "to_id": "a2", "strength": 0.8}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
}

#[tokio::test]
async fn test_execute_tool_memory_traverse() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(1u64)).collect();
    handler
        .execute_tool("memory_inject", json!({"id": "tr1", "vector": vec_data}))
        .await
        .unwrap();
    let result = handler
        .execute_tool(
            "memory_traverse",
            json!({"start_id": "tr1", "depth": 2, "min_strength": 0.0}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert!(result["nodes"].is_array());
}

#[tokio::test]
async fn test_execute_tool_memory_shortest_path() {
    let handler = McpHandler::new(None);
    let vec_data: Vec<Value> = (0..80).map(|_| json!(1u64)).collect();
    handler
        .execute_tool(
            "memory_inject",
            json!({"id": "sp1", "vector": vec_data.clone()}),
        )
        .await
        .unwrap();
    handler
        .execute_tool("memory_inject", json!({"id": "sp2", "vector": vec_data}))
        .await
        .unwrap();
    let result = handler
        .execute_tool(
            "memory_shortest_path",
            json!({"from_id": "sp1", "to_id": "sp2"}),
        )
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
}

#[tokio::test]
async fn test_execute_tool_memory_stats() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool("memory_stats", json!({}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    assert!(result["stats"].is_object());
}

#[tokio::test]
async fn test_execute_tool_memory_export_json() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool("memory_export", json!({}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    let file = result["file"].as_str().unwrap();
    assert!(file.starts_with("export_"));
    assert!(file.ends_with(".json"));
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn test_execute_tool_memory_export_binary() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_tool("memory_export", json!({"format": "binary"}))
        .await
        .unwrap();
    assert_eq!(result["status"], "ok");
    let file = result["file"].as_str().unwrap();
    assert!(file.ends_with(".binary"));
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn test_execute_tool_list_gaps_no_persistence() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_tool("memory_list_gaps", json!({}))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Persistence not enabled"));
}

// --- execute_read_resource tests ---

#[tokio::test]
async fn test_execute_read_resource_stats() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_read_resource("stats://current")
        .await
        .unwrap();
    assert!(result.is_object());
}

#[tokio::test]
async fn test_execute_read_resource_health() {
    let handler = McpHandler::new(None);
    let result = handler
        .execute_read_resource("health://current")
        .await
        .unwrap();
    assert_eq!(result["status"], "healthy");
}

#[tokio::test]
async fn test_execute_read_resource_concept_not_found() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_read_resource("concept://nonexistent_id")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Concept not found"));
}

#[tokio::test]
async fn test_execute_read_resource_unknown_uri() {
    let handler = McpHandler::new(None);
    let err = handler
        .execute_read_resource("unknown://something")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Unknown resource URI"));
}
