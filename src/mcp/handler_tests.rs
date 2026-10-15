//! Crate-internal coverage for the MCP surface: the identity and capabilities
//! the handshake advertises, the tool and resource listings, the tool dispatch,
//! the resource URIs, the JSON schemas, and the two handler helpers.
//!
//! These assertions live in the **lib** target deliberately. The mutation
//! profile runs `--lib` only (`scripts/mutation_test.sh`), so an assertion that
//! lives in `tests/**` cannot kill a mutant in `src/mcp/**` — the sharpness rule
//! in `AGENTS.md` step 11. `tests/mcp_sse_integration.rs` still owns the
//! end-to-end HTTP path; this module owns the crate-internal one.
//!
//! Parent: `src/mcp/handler.rs`. Sibling coverage stays where it is:
//! `tools_tests.rs` owns `parse_hvec*` and the inject/probe/get/delete path,
//! `server_tests.rs` owns the transport exit path.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{McpHandler, res_def, tool_def};
use crate::mcp::schema;
use crate::metadata_filter::MetadataFilter;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rmcp::handler::server::ServerHandler;
use rmcp::model::ErrorCode;
use serde_json::{Value, json};

/// A handler over an in-memory, persistence-less framework — the same shape
/// `tools_tests.rs` builds, repeated because the two modules sit under
/// different parents.
pub(super) async fn handler_with_framework() -> McpHandler {
    let handler = McpHandler::new(None);
    let fw = crate::framework::ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    assert!(handler.framework.set(fw).is_ok());
    handler
}

/// A zero hypervector in the canonical wire form (`HVec10240::to_bytes()`).
fn vector_b64() -> String {
    STANDARD.encode(crate::hyperdim::HVec10240::zero().to_bytes())
}

/// Deletes the file when the returned guard drops, so a failing assertion still
/// cleans up after the `memory_export` tool.
fn remove_on_drop(path: &str) -> RemoveOnDrop {
    RemoveOnDrop(std::path::PathBuf::from(path))
}

struct RemoveOnDrop(std::path::PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The advertised schema for each tool must name the arguments the dispatch
/// actually reads, and require exactly the ones it errors on when absent.
#[test]
fn every_advertised_tool_schema_names_and_requires_its_arguments() {
    // (tool, schema fn, declared properties, required properties)
    let cases: &[(&str, fn() -> Value, &[&str], &[&str])] = &[
        (
            "memory_inject",
            schema::inject_schema,
            &["concept_id", "vector", "metadata"],
            &["concept_id", "vector"],
        ),
        (
            "memory_inject_text",
            schema::inject_text_schema,
            &["concept_id", "text", "metadata"],
            &["concept_id", "text"],
        ),
        (
            "memory_probe",
            schema::probe_schema,
            &["vector", "top_k"],
            &["vector"],
        ),
        (
            "memory_probe_text",
            schema::probe_text_schema,
            &["query", "top_k"],
            &["query"],
        ),
        (
            "memory_probe_filtered",
            schema::probe_filtered_schema,
            &["text", "top_k", "filter"],
            &["text"],
        ),
        (
            "memory_get",
            schema::get_schema,
            &["concept_id"],
            &["concept_id"],
        ),
        (
            "memory_delete",
            schema::delete_schema,
            &["concept_id"],
            &["concept_id"],
        ),
        (
            "memory_associate",
            schema::associate_schema,
            &["from_id", "to_id", "strength"],
            &["from_id", "to_id"],
        ),
        (
            "memory_traverse",
            schema::traverse_schema,
            &["start_id", "depth", "min_strength"],
            &["start_id"],
        ),
        (
            "memory_shortest_path",
            schema::shortest_path_schema,
            &["from_id", "to_id", "weighted"],
            &["from_id", "to_id"],
        ),
        ("memory_stats", schema::stats_schema, &[], &[]),
        ("memory_export", schema::export_schema, &["format"], &[]),
        (
            "memory_list_gaps",
            schema::list_gaps_schema,
            &["min_attempts"],
            &[],
        ),
    ];

    for (tool, schema_fn, properties, required) in cases {
        let value = schema_fn();
        assert_eq!(value["type"], "object", "{tool}: schema must be an object");
        let declared = value["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{tool}: properties must be an object, got {value}"));
        for property in *properties {
            assert!(
                declared.contains_key(*property),
                "{tool}: schema must declare the `{property}` property (has {:?})",
                declared.keys().collect::<Vec<_>>()
            );
        }
        assert_eq!(
            declared.len(),
            properties.len(),
            "{tool}: unexpected properties in {declared:?}"
        );

        let required_names: Vec<&str> = value
            .get("required")
            .and_then(Value::as_array)
            .map(|names| names.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for name in *required {
            assert!(
                required_names.contains(name),
                "{tool}: `{name}` must be required (got {required_names:?})"
            );
        }
        assert_eq!(
            required_names.len(),
            required.len(),
            "{tool}: unexpected required set {required_names:?}"
        );
    }
}

/// `tool_def` and `res_def` are the only constructors for what `list_tools` and
/// `list_resources` advertise, so their fields are the wire contract.
#[test]
fn tool_and_resource_definitions_carry_their_identity_and_schema() {
    let tool = tool_def(
        "memory_example",
        "Example description",
        json!({"type": "object", "properties": {"a": {"type": "string"}}}),
    );
    assert_eq!(tool.name, "memory_example");
    assert_eq!(tool.description.as_deref(), Some("Example description"));
    assert_eq!(tool.input_schema["type"], "object");
    assert!(tool.input_schema["properties"]["a"].is_object());

    let resource = res_def(
        "concept://{id}",
        "Concept by ID",
        "JSON of one concept",
        "application/json",
    );
    assert_eq!(resource.uri, "concept://{id}");
    assert_eq!(resource.name, "Concept by ID");
    assert_eq!(resource.description.as_deref(), Some("JSON of one concept"));
    assert_eq!(resource.mime_type.as_deref(), Some("application/json"));
}

/// A tool error must reach the client as an internal error carrying the cause,
/// not as an empty payload.
#[test]
fn map_error_carries_the_cause_as_an_internal_error() {
    let data = McpHandler::map_error(anyhow::anyhow!("framework said no"));
    assert_eq!(data.code, ErrorCode::INTERNAL_ERROR);
    assert_eq!(data.message, "framework said no");
    assert!(data.data.is_none());
}

/// `get_info` is what the MCP handshake returns to every client.
#[test]
fn get_info_advertises_the_crate_identity_and_its_capabilities() {
    let handler = McpHandler::new(None);
    let info = handler.get_info();

    assert_eq!(info.server_info.name, "chaotic_semantic_memory");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(
        info.capabilities
            .tools
            .as_ref()
            .and_then(|t| t.list_changed),
        Some(true)
    );
    assert_eq!(
        info.capabilities
            .resources
            .as_ref()
            .and_then(|r| r.subscribe),
        Some(false)
    );
    assert_eq!(
        info.capabilities
            .resources
            .as_ref()
            .and_then(|r| r.list_changed),
        Some(true)
    );
}

/// Every advertised tool must have a dispatch arm. A deleted arm falls through
/// to `Tool not implemented`, so both the success shape and the error text are
/// asserted rather than "it did not panic".
#[tokio::test]
async fn execute_tool_dispatches_every_advertised_tool() {
    let handler = handler_with_framework().await;

    for (id, text) in [("alpha", "alpha one"), ("beta", "beta two")] {
        handler
            .execute_tool(
                "memory_inject_text",
                json!({"concept_id": id, "text": text, "metadata": {"kind": "note"}}),
            )
            .await
            .unwrap();
    }
    handler
        .execute_tool(
            "memory_inject_text",
            json!({"concept_id": "gamma", "text": "gamma three"}),
        )
        .await
        .unwrap();
    handler
        .execute_tool(
            "memory_associate",
            json!({"from_id": "alpha", "to_id": "beta", "strength": 0.9}),
        )
        .await
        .unwrap();
    handler
        .execute_tool(
            "memory_associate",
            json!({"from_id": "beta", "to_id": "gamma", "strength": 0.8}),
        )
        .await
        .unwrap();

    let probe = handler
        .execute_tool("memory_probe", json!({"vector": vector_b64(), "top_k": 3}))
        .await
        .unwrap();
    assert_eq!(probe["status"], "ok");
    assert!(probe["results"].is_array(), "{probe}");

    // Retrieval may abstain, so both dispatch statuses are accepted; what must
    // not happen is the unknown-tool error a deleted arm produces.
    let probe_text = handler
        .execute_tool(
            "memory_probe_text",
            json!({"query": "alpha one", "top_k": 3}),
        )
        .await
        .unwrap();
    assert!(
        matches!(probe_text["status"].as_str(), Some("ok" | "abstained")),
        "{probe_text}"
    );

    let filter = serde_json::to_value(MetadataFilter::eq("kind", "note")).unwrap();
    let probe_filtered = handler
        .execute_tool(
            "memory_probe_filtered",
            json!({"text": "alpha one", "top_k": 3, "filter": filter}),
        )
        .await
        .unwrap();
    assert!(
        matches!(probe_filtered["status"].as_str(), Some("ok" | "abstained")),
        "{probe_filtered}"
    );

    let traverse = handler
        .execute_tool("memory_traverse", json!({"start_id": "alpha", "depth": 3}))
        .await
        .unwrap();
    assert!(traverse["nodes"].is_array(), "{traverse}");

    let shortest = handler
        .execute_tool(
            "memory_shortest_path",
            json!({"from_id": "alpha", "to_id": "gamma"}),
        )
        .await
        .unwrap();
    assert!(shortest["path"].is_array(), "{shortest}");

    let stats = handler
        .execute_tool("memory_stats", json!({}))
        .await
        .unwrap();
    assert!(stats["stats"].is_object(), "{stats}");

    // `memory_export` writes into the process working directory, so assert the
    // format branch really produced that format and clean up afterwards.
    for format in ["json", "binary"] {
        let exported = handler
            .execute_tool("memory_export", json!({"format": format}))
            .await
            .unwrap();
        let path = exported["file"].as_str().unwrap();
        // The tool writes into the process working directory, so the cleanup is
        // attached to the guard rather than run after the assertions: a mutant
        // that makes an assertion fail must not also leave the artifact behind
        // (`.gitignore` only covers `export.json`, not `export_<epoch>.json`).
        let _cleanup = remove_on_drop(path);
        assert!(path.ends_with(&format!(".{format}")), "{exported}");
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{path} not written: {e}"));
        let parsed = serde_json::from_slice::<Value>(&bytes);
        if format == "binary" {
            assert!(parsed.is_err(), "`format: binary` must not write JSON");
        } else {
            assert!(parsed.is_ok(), "`format: json` must write JSON");
        }
    }

    // No persistence in this framework, so the arm must report exactly that.
    let gaps = handler
        .execute_tool("memory_list_gaps", json!({"min_attempts": 1}))
        .await
        .unwrap_err();
    assert!(
        gaps.to_string().contains("Persistence not enabled"),
        "{gaps}"
    );
}

/// Argument validation runs *inside* the arm, so the message names the missing
/// argument — which is also what distinguishes a live arm from a deleted one.
#[tokio::test]
async fn each_tool_arm_rejects_its_own_missing_arguments() {
    let handler = handler_with_framework().await;
    let cases: &[(&str, Value, &str)] = &[
        ("memory_probe_text", json!({}), "Missing query"),
        ("memory_probe_filtered", json!({}), "Missing text"),
        (
            "memory_probe_filtered",
            json!({"text": "q"}),
            "Missing filter",
        ),
        ("memory_associate", json!({"from_id": "a"}), "Missing to_id"),
        ("memory_associate", json!({"to_id": "b"}), "Missing from_id"),
        ("memory_traverse", json!({}), "Missing start_id"),
        (
            "memory_shortest_path",
            json!({"from_id": "a"}),
            "Missing to_id",
        ),
    ];

    for (tool, args, expected) in cases {
        let error = handler
            .execute_tool(tool, args.clone())
            .await
            .expect_err(&format!("{tool} {args} must be rejected"));
        assert!(
            error.to_string().contains(expected),
            "{tool} {args}: expected `{expected}`, got `{error}`"
        );
    }

    let unknown = handler
        .execute_tool("memory_not_a_tool", json!({}))
        .await
        .unwrap_err();
    assert!(
        unknown.to_string().contains("Tool not implemented"),
        "{unknown}"
    );
}

/// The three resource URIs the handler advertises, plus the rejection paths.
#[tokio::test]
async fn read_resource_serves_every_advertised_uri_and_rejects_unknown_ones() {
    let handler = handler_with_framework().await;
    handler
        .execute_tool(
            "memory_inject_text",
            json!({"concept_id": "alpha", "text": "alpha one"}),
        )
        .await
        .unwrap();

    let concept = handler
        .execute_read_resource("concept://alpha")
        .await
        .unwrap();
    assert!(concept.is_object(), "{concept}");

    let missing = handler
        .execute_read_resource("concept://nope")
        .await
        .unwrap_err();
    assert!(
        missing.to_string().contains("Concept not found"),
        "{missing}"
    );

    let stats = handler
        .execute_read_resource("stats://current")
        .await
        .unwrap();
    assert!(stats.is_object(), "{stats}");

    match handler.execute_read_resource("health://current").await {
        Ok(health) => assert_eq!(health["status"], "healthy"),
        // A persistence-less framework may have no health backend; the branch is
        // still pinned because an unknown URI would report a different error.
        Err(error) => assert!(
            !error.to_string().contains("Unknown resource URI"),
            "{error}"
        ),
    }

    let unknown = handler
        .execute_read_resource("nonsense://x")
        .await
        .unwrap_err();
    assert!(
        unknown.to_string().contains("Unknown resource URI"),
        "{unknown}"
    );
}
