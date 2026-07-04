#![cfg(feature = "mcp")]

use serde_json::json;
use std::net::SocketAddr;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn test_sse_transport_lifecycle() {
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    drop(listener); // Free the port for the server to bind

    // Start server in background
    let server_handle = tokio::spawn(async move {
        chaotic_semantic_memory::mcp::serve(chaotic_semantic_memory::mcp::McpConfig {
            transport: chaotic_semantic_memory::mcp::Transport::Sse { bind: actual_addr },
            bind: Some(actual_addr.to_string()),
            database: None,
        })
        .await
        .unwrap();
    });

    // Give server a moment to start
    sleep(Duration::from_millis(500)).await;

    // Check if server is still running
    if server_handle.is_finished() {
        // If it finished early, it likely failed or returned Ok(()) prematurely
        server_handle.await.unwrap(); // This will panic if it returned error
        panic!("Server finished prematurely");
    }

    let client = reqwest::Client::new();
    let base_url = format!("http://{actual_addr}");

    // 1. Initial Handshake (POST)
    // We expect the server to return a session ID in the header
    let init_resp = client
        .post(&base_url)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(init_resp.status(), reqwest::StatusCode::OK);
    let session_id = init_resp
        .headers()
        .get("mcp-session-id")
        .expect("Missing session ID header")
        .to_str()
        .unwrap()
        .to_string();

    // 2. List tools
    let list_tools_resp = client
        .post(&base_url)
        .header("mcp-session-id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(list_tools_resp.status(), reqwest::StatusCode::OK);
    let list_tools_body = list_tools_resp.text().await.unwrap();
    assert!(list_tools_body.contains("memory_inject"));
    assert!(list_tools_body.contains("memory_stats"));

    // 3. Call a tool (memory_stats)
    let call_tool_resp = client
        .post(&base_url)
        .header("mcp-session-id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "memory_stats",
                "arguments": {}
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(call_tool_resp.status(), reqwest::StatusCode::OK);
    let call_tool_body = call_tool_resp.text().await.unwrap();
    assert!(call_tool_body.contains("stats"));
    assert!(call_tool_body.contains("concept_count"));

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_mcp_retrieval_abstention() {
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    drop(listener);

    let server_handle = tokio::spawn(async move {
        chaotic_semantic_memory::mcp::serve(chaotic_semantic_memory::mcp::McpConfig {
            transport: chaotic_semantic_memory::mcp::Transport::Sse { bind: actual_addr },
            bind: Some(actual_addr.to_string()),
            database: None,
        })
        .await
        .unwrap();
    });

    sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let base_url = format!("http://{actual_addr}");

    let init_resp = client
        .post(&base_url)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .unwrap();

    let session_id = init_resp
        .headers()
        .get("mcp-session-id")
        .expect("Missing session ID")
        .to_str()
        .unwrap()
        .to_string();

    // Call memory_probe_text with high min_score on empty database
    let call_resp = client
        .post(&base_url)
        .header("mcp-session-id", &session_id)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "memory_probe_text",
                "arguments": {
                    "text": "some query",
                    "min_score": 0.9
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(call_resp.status(), reqwest::StatusCode::OK);
    let body = call_resp.text().await.unwrap();
    // In SSE transport, the body is encoded in 'data:' lines.
    assert!(
        body.contains(r#"\"status\": \"abstained\""#),
        "Body: {body}"
    );
    assert!(body.contains(r#"\"abstained\": true"#));
    assert!(body.contains(r#"\"query\": \"some query\""#));
    assert!(body.contains(r#"\"threshold\": 0.899999"#));

    server_handle.abort();
}
