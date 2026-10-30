//! Wire-level tests for the MCP handler: a real rmcp server over an in-memory
//! duplex transport, spoken to with newline-delimited JSON-RPC.
//!
//! Sibling of `handler_tests.rs` (both are declared from `src/mcp/handler.rs`),
//! split out to stay under the 500-line gate. These three methods —
//! `initialize`, `list_tools`, `list_resources` — are reachable only through a
//! request, so no amount of direct-call testing can pin them; this is the
//! lib-target counterpart of `tests/mcp_streamable_http_integration.rs` and it runs inside
//! `--lib`, which is the profile the mutation gate uses.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::tests::handler_with_framework;
use serde_json::{Value, json};

/// Drive the real rmcp server over an in-memory duplex transport and speak
/// newline-delimited JSON-RPC to it, so the three `ServerHandler` methods that
/// only a request can reach — `initialize`, `list_tools`, `list_resources` —
/// are asserted on what a client actually receives.
///
/// This is the lib-target counterpart of `tests/mcp_streamable_http_integration.rs`:
/// no port, no HTTP, no sleep, and it runs inside `--lib`.
#[tokio::test]
async fn wire_handshake_lists_every_tool_and_resource() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (server_io, client_io) = tokio::io::duplex(1 << 16);
    let (server_read, server_write) = tokio::io::split(server_io);
    let (client_read, client_write) = tokio::io::split(client_io);
    let client_read = BufReader::new(client_read);

    let handler = handler_with_framework().await;

    /// Send one request line and return the response whose `id` matches.
    async fn request(
        writer: &mut (impl AsyncWriteExt + Unpin),
        reader: &mut (impl AsyncBufReadExt + Unpin),
        line: Value,
    ) -> Value {
        let id = line["id"].clone();
        let mut framed = serde_json::to_string(&line).unwrap();
        framed.push('\n');
        writer.write_all(framed.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();

        loop {
            let mut response = String::new();
            let read = reader.read_line(&mut response).await.unwrap();
            assert!(
                read > 0,
                "the server closed the stream before replying to {id}"
            );
            if response.trim().is_empty() {
                continue;
            }
            let parsed: Value = serde_json::from_str(response.trim())
                .unwrap_or_else(|e| panic!("server wrote a non-JSON line: {response:?} ({e})"));
            if parsed["id"] == id {
                return parsed;
            }
        }
    }

    /// Send one notification line. Notifications carry no `id`, so nothing is
    /// read back — waiting for a reply here is what deadlocks a handshake.
    async fn notify(writer: &mut (impl AsyncWriteExt + Unpin), line: Value) {
        let mut framed = serde_json::to_string(&line).unwrap();
        framed.push('\n');
        writer.write_all(framed.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }

    let client = tokio::spawn(async move {
        let mut client_write = client_write;
        let mut client_read = client_read;
        let discover = request(
            &mut client_write,
            &mut client_read,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "server/discover",
                // SEP-2567: discovery has no session yet, so rmcp requires the
                // lifecycle inline in `_meta`.
                "params": {
                    "_meta": {
                        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                        "io.modelcontextprotocol/clientCapabilities": {}
                    }
                },
            }),
        )
        .await;
        let initialize = request(
            &mut client_write,
            &mut client_read,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2026-07-28",
                    "capabilities": {},
                    "clientInfo": {"name": "wire-test", "version": "0.0.0"},
                },
            }),
        )
        .await;
        let server_info = &initialize["result"]["serverInfo"];
        assert_eq!(
            server_info["name"], "chaotic_semantic_memory",
            "the handshake must report the crate identity: {initialize}"
        );
        assert_eq!(server_info["version"], env!("CARGO_PKG_VERSION"));
        // A client may ask for 2026-07-28 over `initialize`, but that revision
        // has no initialize lifecycle: SEP-2567 moved it into per-request
        // `_meta`, so the server answers the legacy handshake with the newest
        // version that still negotiates this way. The stateless 2026-07-28 path
        // is exercised by the streamable-HTTP integration suite, and the
        // advertisement of both revisions is asserted below.
        assert_eq!(
            initialize["result"]["protocolVersion"], "2025-11-25",
            "a request for a revision without an initialize lifecycle must negotiate down: {initialize}"
        );
        assert_eq!(
            initialize["result"]["capabilities"]["tools"]["listChanged"], true,
            "tools must be advertised as a list-changing capability: {initialize}"
        );
        assert_eq!(
            initialize["result"]["capabilities"]["resources"]["subscribe"],
            false
        );

        notify(
            &mut client_write,
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        )
        .await;

        let tools = request(
            &mut client_write,
            &mut client_read,
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list", "params": {}}),
        )
        .await;
        let resources = request(
            &mut client_write,
            &mut client_read,
            json!({"jsonrpc": "2.0", "id": 4, "method": "resources/list", "params": {}}),
        )
        .await;
        let repeated_tools = request(
            &mut client_write,
            &mut client_read,
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/list", "params": {}}),
        )
        .await;
        (discover, tools, resources, repeated_tools)
    });

    let running = rmcp::serve_server(handler, (server_read, server_write))
        .await
        .expect("the server must accept a duplex transport");

    let (discover, tools, resources, repeated_tools) =
        tokio::time::timeout(std::time::Duration::from_secs(30), client)
            .await
            .expect("the wire exchange must not hang")
            .expect("the client task must not panic");

    let versions: Vec<&str> = discover["result"]["supportedVersions"]
        .as_array()
        .unwrap_or_else(|| panic!("server/discover must return protocol versions: {discover}"))
        .iter()
        .map(|version| version.as_str().unwrap())
        .collect();
    assert!(
        versions.contains(&"2026-07-28"),
        "missing current protocol: {versions:?}"
    );
    assert!(
        versions.contains(&"2025-11-25"),
        "missing fallback protocol: {versions:?}"
    );
    assert_eq!(
        discover["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "chaotic_semantic_memory"
    );
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list must return an array: {tools}"))
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for expected in [
        "memory_inject",
        "memory_inject_text",
        "memory_probe",
        "memory_probe_text",
        "memory_probe_filtered",
        "memory_get",
        "memory_delete",
        "memory_associate",
        "memory_traverse",
        "memory_shortest_path",
        "memory_stats",
        "memory_export",
        "memory_list_gaps",
    ] {
        assert!(
            names.contains(&expected),
            "tools/list must advertise `{expected}` (got {names:?})"
        );
    }
    assert_eq!(names.len(), 13, "unexpected tool count: {names:?}");
    assert_eq!(
        tools["result"]["tools"][0]["inputSchema"]["type"], "object",
        "each tool must carry its own JSON schema: {tools}"
    );
    assert_eq!(
        tools["result"]["tools"], repeated_tools["result"]["tools"],
        "tools/list must preserve its deterministic order across requests"
    );

    let uris: Vec<&str> = resources["result"]["resources"]
        .as_array()
        .unwrap_or_else(|| panic!("resources/list must return an array: {resources}"))
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect();
    assert_eq!(
        uris,
        vec!["concept://{id}", "stats://current", "health://current"],
        "resources/list must advertise every handler URI"
    );

    // The client task has finished, so it has dropped both halves of the duplex
    // and the server's transport has seen EOF. Require it to stop within a bound:
    // a fixture that leaks a running server would turn the suite into a wait
    // instead. The `QuitReason` itself is rmcp's business, deliberately not
    // asserted.
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(10), running.waiting()).await;
    let _ = stopped
        .expect("the server must stop once the client half of the duplex is closed")
        .expect("the server task must not panic");
}
