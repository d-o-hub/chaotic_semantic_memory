//! MCP Server Handler (ADR-0067)
//!
//! Combined tool and resource handler implementing rmcp::ServerHandler.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use serde_json::{Value, json};
use tokio::sync::OnceCell;
use tracing::{error, info};

use super::schema;
use crate::framework::ChaoticSemanticFramework;

/// Combined MCP handler for chaotic_semantic_memory.
pub struct McpHandler {
    pub(crate) database: Option<PathBuf>,
    pub(crate) framework: OnceCell<ChaoticSemanticFramework>,
}

impl McpHandler {
    /// Check if the framework is initialized (for testing).
    #[cfg(test)]
    pub fn is_framework_initialized(&self) -> bool {
        self.framework.get().is_some()
    }
}

impl McpHandler {
    /// Create a new MCP handler.
    pub const fn new(database: Option<PathBuf>) -> Self {
        Self {
            database,
            framework: OnceCell::const_new(),
        }
    }

    /// Get or initialize the framework instance.
    async fn framework(&self) -> Result<&ChaoticSemanticFramework> {
        self.framework
            .get_or_try_init(|| async {
                info!("Initializing ChaoticSemanticFramework for MCP");
                match crate::cli::commands::create_framework(self.database.as_deref()).await {
                    Ok(fw) => Ok(fw),
                    Err(e) => {
                        error!("Failed to initialize framework: {e}");
                        Err(anyhow::anyhow!("Failed to initialize framework: {e}"))
                    }
                }
            })
            .await
    }

    fn map_error(e: anyhow::Error) -> ErrorData {
        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
    }
}

impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability {
            list_changed: Some(true),
        });
        caps.resources = Some(ResourcesCapability {
            subscribe: Some(false),
            list_changed: Some(true),
        });
        InitializeResult::new(caps).with_server_info(Implementation::new(
            "chaotic_semantic_memory",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = vec![
            tool_def(
                "memory_inject",
                "Store a concept with optional metadata",
                schema::inject_schema(),
            ),
            tool_def(
                "memory_inject_text",
                "Store a concept from text",
                schema::inject_text_schema(),
            ),
            tool_def(
                "memory_probe",
                "Similarity search by vector ID",
                schema::probe_schema(),
            ),
            tool_def(
                "memory_probe_text",
                "Text-query similarity search",
                schema::probe_text_schema(),
            ),
            tool_def(
                "memory_probe_filtered",
                "Metadata-filtered search",
                schema::probe_filtered_schema(),
            ),
            tool_def("memory_get", "Fetch concept by ID", schema::get_schema()),
            tool_def("memory_delete", "Remove a concept", schema::delete_schema()),
            tool_def(
                "memory_associate",
                "Create directed association",
                schema::associate_schema(),
            ),
            tool_def(
                "memory_traverse",
                "BFS graph traversal",
                schema::traverse_schema(),
            ),
            tool_def(
                "memory_shortest_path",
                "Find path between concepts",
                schema::shortest_path_schema(),
            ),
            tool_def("memory_stats", "DB stats snapshot", schema::stats_schema()),
            tool_def("memory_export", "Export to JSON", schema::export_schema()),
        ];
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    #[allow(clippy::unwrap_used)]
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let name = request.name.as_ref();
        let args = request.arguments.map_or(Value::Null, Value::Object);

        let result = self
            .execute_tool(name, args)
            .await
            .map_err(Self::map_error)?;

        Ok(CallToolResult::success(vec![Content::new(
            RawContent::text(serde_json::to_string_pretty(&result).unwrap()),
            None,
        )]))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let resources = vec![
            res_def(
                "concept://{id}",
                "Concept by ID",
                "JSON of one concept",
                "application/json",
            ),
            res_def(
                "stats://current",
                "Current stats",
                "Live stats snapshot",
                "application/json",
            ),
            res_def(
                "health://current",
                "Health check",
                "Persistence health status",
                "application/json",
            ),
        ];
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    #[allow(clippy::unwrap_used)]
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = request.uri.clone();
        let result = self
            .execute_read_resource(uri.as_ref())
            .await
            .map_err(Self::map_error)?;

        Ok(ReadResourceResult::new(vec![
            ResourceContents::TextResourceContents {
                uri,
                mime_type: Some("application/json".to_string()),
                text: serde_json::to_string_pretty(&result).unwrap(),
                meta: None,
            },
        ]))
    }
}

impl McpHandler {
    #[allow(clippy::unwrap_used)]
    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let fw = self.framework().await?;
        match name {
            "memory_inject" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                let vec_data = args["vector"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing vector"))?;
                let vector = parse_hvec(vec_data)?;
                if let Some(meta) = args.get("metadata") {
                    let meta_map: HashMap<String, Value> = serde_json::from_value(meta.clone())?;
                    fw.inject_concept_with_metadata(id, vector, meta_map)
                        .await?;
                } else {
                    fw.inject_concept(id, vector).await?;
                }
                Ok(json!({"status": "ok", "id": id}))
            }
            "memory_inject_text" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;
                if let Some(meta) = args.get("metadata") {
                    let meta_map: HashMap<String, Value> = serde_json::from_value(meta.clone())?;
                    fw.inject_text_with_metadata(id, text, meta_map).await?;
                } else {
                    fw.inject_text(id, text).await?;
                }
                Ok(json!({"status": "ok", "id": id}))
            }
            "memory_probe" => {
                let vec_data = args["vector"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing vector"))?;
                let vector = parse_hvec(vec_data)?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let results = fw.probe(vector, top_k).await?;
                Ok(json!({"status": "ok", "results": results}))
            }
            "memory_probe_text" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query"))?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let results = fw.probe_text(query, top_k).await?;
                Ok(json!({"status": "ok", "results": results}))
            }
            "memory_probe_filtered" => {
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text"))?;
                let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
                let filter_val = args
                    .get("filter")
                    .ok_or_else(|| anyhow::anyhow!("Missing filter"))?;
                let filter: crate::metadata_filter::MetadataFilter =
                    serde_json::from_value(filter_val.clone())?;

                let results = fw.probe_text_filtered(text, top_k, &filter).await?;
                Ok(json!({"status": "ok", "results": results}))
            }
            "memory_get" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                if let Some(concept) = fw.get_concept(id).await? {
                    Ok(json!({"status": "ok", "concept": concept}))
                } else {
                    Ok(json!({"status": "not_found"}))
                }
            }
            "memory_delete" => {
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id"))?;
                fw.delete_concept(id).await?;
                Ok(json!({"status": "ok", "deleted": true}))
            }
            "memory_associate" => {
                let from = args["from_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing from_id"))?;
                let to = args["to_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing to_id"))?;
                let strength = args["strength"].as_f64().unwrap_or(0.5) as f32;
                fw.associate(from, to, strength).await?;
                Ok(json!({"status": "ok"}))
            }
            "memory_traverse" => {
                let start_id = args["start_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing start_id"))?;
                let depth = args["depth"].as_u64().unwrap_or(3) as usize;
                let min_strength = args["min_strength"].as_f64().unwrap_or(0.0) as f32;
                let config = crate::graph_traversal::TraversalConfig {
                    max_depth: depth,
                    min_strength,
                    max_results: 100,
                };
                let results = fw.traverse(start_id, config).await?;
                Ok(json!({"status": "ok", "nodes": results}))
            }
            "memory_shortest_path" => {
                let from = args["from_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing from_id"))?;
                let to = args["to_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing to_id"))?;
                let path = fw.shortest_path(from, to).await?;
                Ok(json!({"status": "ok", "path": path}))
            }
            "memory_stats" => {
                let stats = fw.stats().await?;
                Ok(json!({"status": "ok", "stats": stats}))
            }
            "memory_export" => {
                let format = args["format"].as_str().unwrap_or("json");
                let path = format!(
                    "export_{}.{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    format
                );
                if format == "binary" {
                    fw.export_binary(&path).await?;
                } else {
                    fw.export_json(&path).await?;
                }
                Ok(json!({"status": "ok", "file": path}))
            }
            _ => Err(anyhow::anyhow!("Tool not implemented: {name}")),
        }
    }

    async fn execute_read_resource(&self, uri: &str) -> Result<Value> {
        let fw = self.framework().await?;
        if let Some(id) = uri.strip_prefix("concept://") {
            if let Some(concept) = fw.get_concept(id).await? {
                Ok(json!(concept))
            } else {
                Err(anyhow::anyhow!("Concept not found: {id}"))
            }
        } else if uri == "stats://current" {
            let stats = fw.stats().await?;
            Ok(json!(stats))
        } else if uri == "health://current" {
            fw.persistence_health_check().await?;
            Ok(json!({"status": "healthy"}))
        } else {
            Err(anyhow::anyhow!("Unknown resource URI: {uri}"))
        }
    }
}
#[allow(clippy::unwrap_used)]
fn tool_def(name: &str, desc: &str, schema: Value) -> Tool {
    Tool::new(
        name.to_string(),
        desc.to_string(),
        Arc::new(schema.as_object().unwrap().clone()),
    )
}

fn res_def(uri: &str, name: &str, desc: &str, mime: &str) -> Resource {
    Resource::new(
        RawResource {
            uri: uri.to_string(),
            name: name.to_string(),
            title: None,
            description: Some(desc.to_string()),
            mime_type: Some(mime.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        None,
    )
}

fn parse_hvec(vec_data: &[Value]) -> Result<csm_core::hyperdim::HVec10240> {
    if vec_data.len() != 80 {
        return Err(anyhow::anyhow!("Vector must have 80 elements"));
    }
    let mut data = [0u128; 80];
    for (i, val) in vec_data.iter().enumerate() {
        data[i] = val
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid vector element"))? as u128;
    }
    Ok(csm_core::hyperdim::HVec10240 { data })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_hvec_roundtrips_values() {
        let input: Vec<Value> = (0u64..80).map(|i| json!(i * 3 + 7)).collect();
        let hvec = parse_hvec(&input).expect("must parse successfully");
        for (i, val) in hvec.data.iter().enumerate() {
            assert_eq!(*val, (i as u128) * 3 + 7, "element {i} mismatch");
        }
    }

    #[test]
    fn parse_hvec_rejects_wrong_length() {
        let short: Vec<Value> = vec![json!(1u64); 10];
        assert!(
            parse_hvec(&short).is_err(),
            "must reject non-80-element input"
        );
    }

    #[test]
    fn parse_hvec_known_values_match_expected() {
        let input: Vec<Value> = vec![json!(0u64), json!(1), json!(255), json!(u64::MAX)]
            .into_iter()
            .chain((4..80).map(|i| json!(i as u64)))
            .collect();
        let hvec = parse_hvec(&input).expect("must parse");
        assert_eq!(hvec.data[0], 0u128, "first element must be 0");
        assert_eq!(hvec.data[1], 1u128, "second element must be 1");
        assert_eq!(hvec.data[2], 255u128, "third element must be 255");
        assert_eq!(
            hvec.data[3],
            u64::MAX as u128,
            "fourth element must be u64::MAX"
        );
        assert_eq!(hvec.data[79], 79u128, "last element must be 79");
    }

    #[test]
    fn parse_hvec_all_zeros_roundtrips() {
        let input: Vec<Value> = (0..80).map(|_| json!(0u64)).collect();
        let hvec = parse_hvec(&input).expect("must parse zeros");
        assert!(
            hvec.data.iter().all(|&v| v == 0),
            "all elements must be zero"
        );
    }

    #[test]
    fn parse_hvec_rejects_non_numeric() {
        let mut input: Vec<Value> = (0..80).map(|i| json!(i as u64)).collect();
        input[40] = json!("not a number");
        assert!(
            parse_hvec(&input).is_err(),
            "must reject non-numeric element"
        );
    }

    #[test]
    fn test_mcp_handler_clone_initializes_separate_framework() {
        let db_path = Some(PathBuf::from("test_mcp_clone.db"));
        let handler1 = McpHandler::new(db_path.clone());
        let handler2 = handler1.clone();

        assert!(!handler1.is_framework_initialized());
        assert!(!handler2.is_framework_initialized());

        assert_eq!(handler1.database, db_path);
        assert_eq!(handler2.database, db_path);
    }
}
