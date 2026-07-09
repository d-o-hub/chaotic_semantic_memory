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
