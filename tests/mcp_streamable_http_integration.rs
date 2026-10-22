#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "mcp")]

use chaotic_semantic_memory::mcp::{McpConfig, Transport, serve_with_shutdown};
use serde_json::json;
use std::net::SocketAddr;

#[tokio::test]
async fn legacy_protocol_uses_streamable_http_sessions() {
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    drop(listener); // Free the port for the server to bind

    // This legacy protocol version exercises compatibility-only sessions. The
    // 2026-07-28 conformance test below verifies the current stateless path.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        serve_with_shutdown(
            McpConfig {
                transport: Transport::StreamableHttp { bind: actual_addr },
                bind: Some(actual_addr.to_string()),
                database: None,
                auth_token: None,
                allowed_hosts: None,
                allowed_origins: Vec::new(),
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

    // Legacy negotiation retains a session ID for this compatibility version.
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
        .expect("the Streamable HTTP server task must not panic")
        .expect("serve must return Ok(()) once the shutdown signal resolves");
}

#[tokio::test]
async fn current_streamable_http_is_stateless_and_enforces_access_policy() {
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        serve_with_shutdown(
            McpConfig {
                transport: Transport::StreamableHttp { bind: actual_addr },
                bind: Some(actual_addr.to_string()),
                database: None,
                auth_token: Some("wire-test-secret".to_string()),
                allowed_hosts: None,
                allowed_origins: Vec::new(),
                ttl_cleanup_interval: 0,
            },
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
    });

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
    let discover = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    // SEP-2243 makes `Mcp-Method` mandatory from protocol version 2026-07-28 and
    // requires it to equal the body method; rmcp answers a missing or mismatched
    // value with JSON-RPC -32020 over HTTP 400.
    let unauthenticated = client
        .post(&base_url)
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&discover)
        .send()
        .await
        .unwrap();
    assert_eq!(
        unauthenticated.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "configured bearer auth must cover discovery requests"
    );

    let forbidden_host = client
        .post(&base_url)
        .header("Authorization", "Bearer wire-test-secret")
        .header("Host", "attacker.invalid")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&discover)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden_host.status(), reqwest::StatusCode::FORBIDDEN);

    let forbidden_origin = client
        .post(&base_url)
        .header("Authorization", "Bearer wire-test-secret")
        .header("Origin", "https://attacker.invalid")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&discover)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden_origin.status(), reqwest::StatusCode::FORBIDDEN);

    let discovered = client
        .post(&base_url)
        .header("Authorization", "Bearer wire-test-secret")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .header(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        )
        .json(&discover)
        .send()
        .await
        .unwrap();
    let discovery_status = discovered.status();
    let sessionless = discovered.headers().get("mcp-session-id").is_none();
    let discovery_body = discovered.text().await.unwrap();
    assert_eq!(
        discovery_status,
        reqwest::StatusCode::OK,
        "discovery must succeed, body: {discovery_body}"
    );
    assert!(sessionless, "stateless discovery must not open a session");
    let discovery: serde_json::Value = serde_json::from_str(&discovery_body).unwrap();
    assert!(
        discovery["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|version| version == "2026-07-28")
    );

    let list_tools = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let list = client
        .post(&base_url)
        .header("Authorization", "Bearer wire-test-secret")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&list_tools)
        .send()
        .await
        .unwrap();
    let list_status = list.status();
    let list_sessionless = list.headers().get("mcp-session-id").is_none();
    let list_body = list.text().await.unwrap();
    assert_eq!(
        list_status,
        reqwest::StatusCode::OK,
        "stateless tools/list must succeed, body: {list_body}"
    );
    assert!(
        list_sessionless,
        "stateless tools/list must not open a session"
    );
    let list_json: serde_json::Value = serde_json::from_str(&list_body).unwrap();
    let repeated = client
        .post(&base_url)
        .header("Authorization", "Bearer wire-test-secret")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/list")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {}
                }
            }
        }))
        .send()
        .await
        .unwrap();
    let repeated_status = repeated.status();
    let repeated_body = repeated.text().await.unwrap();
    assert_eq!(
        repeated_status,
        reqwest::StatusCode::OK,
        "the repeated tools/list must succeed, body: {repeated_body}"
    );
    let repeated_json: serde_json::Value = serde_json::from_str(&repeated_body).unwrap();
    assert_eq!(
        list_json["result"]["tools"], repeated_json["result"]["tools"],
        "tool ordering must be deterministic"
    );

    drop(client);
    shutdown_tx.send(()).unwrap();
    server
        .await
        .expect("the Streamable HTTP server task must not panic")
        .expect("Streamable HTTP must stop cooperatively");
}
