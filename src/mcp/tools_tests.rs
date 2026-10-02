//! Tests for the MCP tool surface (parent `src/mcp/tools.rs`).
//!
//! Extracted from the parent module to stay under the 500-line gate
//! (AGENTS.md step 9: child-module extraction over comment stripping).
//! The bodies are verbatim; only the indentation was dedented.

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
/// The MCP server path must stop the framework's TTL cleanup task at exit
/// (ADR-0099). `shutdown()` is what `mcp::server::serve` calls after the
/// transport ends, so this pins the observable effect: a framework built
/// with a cleanup interval has a live task, and `shutdown()` takes it.
#[tokio::test]
async fn test_handler_shutdown_stops_the_cleanup_task() {
    use crate::framework_ttl_advanced::TtlConfig;

    let handler = McpHandler::new(None);
    let fw = crate::framework::ChaoticSemanticFramework::builder()
        .with_ttl_config(TtlConfig {
            cleanup_interval_seconds: 1,
            ..Default::default()
        })
        .without_persistence()
        .build()
        .await
        .unwrap();
    assert!(handler.framework.set(fw).is_ok());
    let fw = handler.framework.get().expect("framework initialized");
    let task = fw.cleanup.as_ref().expect("cleanup task spawned");

    assert!(
        task.handle.lock().await.is_some(),
        "the task must be live before shutdown"
    );
    handler.shutdown().await.unwrap();
    assert!(
        task.handle.lock().await.is_none(),
        "shutdown must cancel and await the cleanup task"
    );
    // Idempotent: the handle is already gone.
    handler.shutdown().await.unwrap();
}

/// A server that served no request never initializes the framework, so
/// shutdown must be a no-op rather than building one just to stop it.
#[tokio::test]
async fn test_handler_shutdown_is_a_noop_without_a_framework() {
    let handler = McpHandler::new(None);
    handler.shutdown().await.unwrap();
    assert!(
        !handler.is_framework_initialized(),
        "shutdown must not initialize the framework"
    );
}
