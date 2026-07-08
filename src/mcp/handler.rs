//! MCP Server Handler (ADR-0067)
//!
//! Combined tool and resource handler implementing rmcp::ServerHandler.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use serde_json::Value;
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
    pub(crate) async fn framework(&self) -> Result<&ChaoticSemanticFramework> {
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

    pub(crate) fn map_error(e: anyhow::Error) -> ErrorData {
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
            tool_def(
                "memory_list_gaps",
                "List known memory gaps (failed retrieval attempts)",
                schema::list_gaps_schema(),
            ),
        ];
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

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

#[cfg(all(test, feature = "mcp"))]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_handler_new() {
        let handler = McpHandler::new(None);
        assert!(handler.database.is_none());
    }

    #[test]
    fn test_mcp_handler_new_with_database() {
        let handler = McpHandler::new(Some(PathBuf::from("/tmp/test.db")));
        assert_eq!(
            handler.database.as_ref().unwrap().to_str().unwrap(),
            "/tmp/test.db"
        );
    }

    #[test]
    fn test_get_info_has_tools_capability() {
        let handler = McpHandler::new(None);
        let info = handler.get_info();
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_get_info_has_resources_capability() {
        let handler = McpHandler::new(None);
        let info = handler.get_info();
        assert!(info.capabilities.resources.is_some());
    }

    #[test]
    fn test_get_info_server_name() {
        let handler = McpHandler::new(None);
        let info = handler.get_info();
        assert_eq!(info.server_info.name, "chaotic_semantic_memory");
    }

    #[test]
    fn test_get_info_tools_list_changed() {
        let handler = McpHandler::new(None);
        let info = handler.get_info();
        let tools = info.capabilities.tools.unwrap();
        assert_eq!(tools.list_changed, Some(true));
    }

    #[test]
    fn test_get_info_resources_subscribe() {
        let handler = McpHandler::new(None);
        let info = handler.get_info();
        let res = info.capabilities.resources.unwrap();
        assert_eq!(res.subscribe, Some(false));
    }

    #[test]
    fn test_get_info_resources_list_changed() {
        let handler = McpHandler::new(None);
        let info = handler.get_info();
        let res = info.capabilities.resources.unwrap();
        assert_eq!(res.list_changed, Some(true));
    }

    #[test]
    fn test_is_framework_initialized_before() {
        let handler = McpHandler::new(None);
        assert!(!handler.is_framework_initialized());
    }

    #[test]
    fn test_map_error() {
        let err = anyhow::anyhow!("test error message");
        let error_data = McpHandler::map_error(err);
        assert_eq!(error_data.code, ErrorCode::INTERNAL_ERROR);
        assert!(error_data.message.contains("test error message"));
    }

    #[test]
    fn test_map_error_preserves_message() {
        let err = anyhow::anyhow!("specific failure reason");
        let error_data = McpHandler::map_error(err);
        assert_eq!(error_data.message, "specific failure reason");
    }

    #[tokio::test]
    async fn test_framework_initializes() {
        let handler = McpHandler::new(None);
        assert!(!handler.is_framework_initialized());
        let _fw = handler.framework().await.unwrap();
        assert!(handler.is_framework_initialized());
    }
}
