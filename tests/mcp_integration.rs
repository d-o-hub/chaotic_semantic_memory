#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "mcp")]
//! Integration tests for MCP server handler (ADR-0090).
//!
//! Tests the MCP handler construction, server info, capabilities,
//! and configuration types. Tool/resource calls require `RequestContext`
//! which needs a live transport — those are covered by the inline unit
//! tests in `src/mcp/handler.rs`.

use std::path::PathBuf;

use chaotic_semantic_memory::mcp::{McpConfig, McpHandler, Transport};
use rmcp::handler::server::ServerHandler;

// --- Handler Construction ---

#[test]
fn handler_construction_without_database() {
    let _handler = McpHandler::new(None);
}

#[test]
fn handler_construction_with_database_path() {
    let _handler = McpHandler::new(Some(PathBuf::from("/tmp/mcp_test.db")));
}

// --- ServerInfo / get_info ---

#[test]
fn get_info_contains_server_name() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();
    let debug = format!("{info:?}");
    assert!(
        debug.contains("chaotic_semantic_memory"),
        "ServerInfo must contain crate name, got: {debug}"
    );
}

#[test]
fn get_info_contains_version() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();
    let debug = format!("{info:?}");
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        debug.contains(version),
        "ServerInfo must contain version {version}, got: {debug}"
    );
}

#[test]
fn get_info_has_tools_capability() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();
    assert!(
        info.capabilities.tools.is_some(),
        "ServerInfo must advertise tools capability"
    );
}

#[test]
fn get_info_has_resources_capability() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();
    assert!(
        info.capabilities.resources.is_some(),
        "ServerInfo must advertise resources capability"
    );
}

#[test]
fn get_info_tools_list_changed_is_true() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();
    let tools_cap = info.capabilities.tools.as_ref().unwrap();
    assert_eq!(tools_cap.list_changed, Some(true));
}

#[test]
fn get_info_resources_subscribe_is_false() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();
    let res_cap = info.capabilities.resources.as_ref().unwrap();
    assert_eq!(res_cap.subscribe, Some(false));
}

// --- McpConfig ---

#[test]
fn mcp_config_default_uses_stdio_transport() {
    let config = McpConfig::default();
    assert!(matches!(config.transport, Transport::Stdio));
    assert!(config.bind.is_none());
    assert!(config.database.is_none());
}

#[test]
fn mcp_config_with_database() {
    let config = McpConfig {
        transport: Transport::Stdio,
        bind: None,
        database: Some(PathBuf::from("test.db")),
    };
    assert_eq!(
        config.database.as_deref(),
        Some(PathBuf::from("test.db").as_path())
    );
}

// --- Transport enum ---

#[test]
fn transport_debug_format() {
    let t = Transport::Stdio;
    let debug = format!("{t:?}");
    assert!(debug.contains("Stdio"));
}

#[test]
fn transport_clone() {
    let t = Transport::Stdio;
    let t2 = t;
    assert!(matches!(t2, Transport::Stdio));
}

// --- Hypervector wire format (ADR-0094) ---

#[test]
fn mcp_hypervector_base64_preserves_high_bits() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use chaotic_semantic_memory::HVec10240;

    // Full 10240-bit vectors must survive the MCP wire format (base64 of
    // HVec10240::to_bytes()). JSON integers cannot carry bits above 64.
    let mut data = [0u128; 80];
    data[0] = 1u128 << 80;
    data[1] = u128::MAX;
    data[2] = (u64::MAX as u128) << 64 | 0xCAFE_F00D;
    let original = HVec10240 { data };

    let wire = STANDARD.encode(original.to_bytes());
    assert!(
        !wire.is_empty(),
        "MCP vector wire is a non-empty base64 string"
    );

    let restored = HVec10240::from_bytes(
        &STANDARD
            .decode(&wire)
            .expect("MCP wire must be valid standard base64"),
    )
    .expect("decoded bytes must be a valid 1280-byte HVec10240");

    assert_eq!(restored.data[0], 1u128 << 80);
    assert_eq!(restored.data[1], u128::MAX);
    assert_eq!(restored.data[2], (u64::MAX as u128) << 64 | 0xCAFE_F00D);
    assert_eq!(restored.to_bytes(), original.to_bytes());

    // Serde human-readable form must match the same base64 wire (used when
    // concepts are returned from memory_get).
    let json = serde_json::to_value(original).expect("HVec serializes");
    assert_eq!(json.as_str(), Some(wire.as_str()));
    let via_serde: HVec10240 = serde_json::from_value(json).expect("HVec deserializes");
    assert_eq!(via_serde.data[0], 1u128 << 80);
}
