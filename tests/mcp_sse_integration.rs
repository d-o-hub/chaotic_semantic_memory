#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "mcp")]

use chaotic_semantic_memory::mcp::{McpConfig, Transport, serve_with_shutdown};
use serde_json::json;
use std::net::SocketAddr;

#[tokio::test]
async fn test_sse_transport_lifecycle() {
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    drop(listener); // Free the port for the server to bind

    // `serve_with_shutdown` is the entry `serve` itself uses — `serve` is
    // `serve_with_shutdown(config, ctrl_c)` — so the production path, including
    // its ADR-0099 shutdown hop, is what runs here. The oneshot only replaces the
    // operator's keyboard; it is the same `Future<Output = ()>` signal mechanism.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        serve_with_shutdown(
            McpConfig {
                transport: Transport::Sse { bind: actual_addr },
                bind: Some(actual_addr.to_string()),
                database: None,
                ttl_cleanup_interval: 0,
            },
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
    });

    // No startup sleep: the connect retry *is* the readiness synchronisation, and
    // a server that dies while binding reports its own error instead of leaving
    // the test to the harness timeout.
    loop {
        if tokio::net::TcpStream::connect(actual_addr).await.is_ok() {
            break;
        }
        if server.is_finished() {
            let outcome = server.await.expect("the server task must not panic");
            panic!("server stopped before it was listening: {outcome:?}");
        }
        tokio::task::yield_now().await;
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

    // The exit is awaited, never aborted: `abort()` is exactly what hid the fact
    // that `axum::serve` without `with_graceful_shutdown` does not return. The
    // client is dropped first so reqwest's pooled keep-alive socket is closed and
    // graceful shutdown has nothing left to wait for.
    drop(client);
    shutdown_tx.send(()).unwrap();
    server
        .await
        .expect("the SSE server task must not panic")
        .expect("serve must return Ok(()) once the shutdown signal resolves");
}
