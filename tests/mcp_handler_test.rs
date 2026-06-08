//! Integration tests for MCP handler utilities.
//!
//! These tests exercise `parse_hvec` — the JSON→HVec10240 parser used by
//! the MCP server's `memory_inject` and `memory_probe` tools.
//!
//! Run with: `cargo test --features mcp --test mcp_handler_test`

#![cfg(feature = "mcp")]

use chaotic_semantic_memory::mcp::parse_hvec;
use serde_json::json;

#[test]
fn parse_hvec_valid_80_elements() {
    let vals: Vec<serde_json::Value> = (0..80u64).map(|i| json!(i)).collect();
    let hvec = parse_hvec(&vals).expect("should parse valid 80-element vector");
    for (i, &d) in hvec.data.iter().enumerate() {
        assert_eq!(d, i as u128, "element {i} mismatch");
    }
}

#[test]
fn parse_hvec_wrong_length_too_few() {
    let vals: Vec<serde_json::Value> = (0..79u64).map(|i| json!(i)).collect();
    let err = parse_hvec(&vals).unwrap_err();
    assert!(
        err.to_string().contains("80 elements"),
        "expected length error, got: {err}"
    );
}

#[test]
fn parse_hvec_wrong_length_too_many() {
    let vals: Vec<serde_json::Value> = (0..81u64).map(|i| json!(i)).collect();
    let err = parse_hvec(&vals).unwrap_err();
    assert!(
        err.to_string().contains("80 elements"),
        "expected length error, got: {err}"
    );
}

#[test]
fn parse_hvec_empty_array_errors() {
    let vals: Vec<serde_json::Value> = vec![];
    let err = parse_hvec(&vals).unwrap_err();
    assert!(
        err.to_string().contains("80 elements"),
        "expected length error for empty array, got: {err}"
    );
}
