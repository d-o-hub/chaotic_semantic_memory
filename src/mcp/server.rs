use crate::framework::ChaoticSemanticFramework;
use crate::mcp::tools::MemoryTools;
use crate::mcp::resources::MemoryResources;
use crate::mcp::schema::*;
use rmcp::model::{Tool, ErrorData};
use rmcp::Server;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct McpServer {
    pub framework: Arc<ChaoticSemanticFramework>,
}

impl McpServer {
    pub fn new(framework: ChaoticSemanticFramework) -> Self {
        Self {
            framework: Arc::new(framework),
        }
    }

    pub async fn run_stdio(&self) -> Result<(), ErrorData> {
        let server = self.build_server();
        server.run_stdio().await
    }

    pub async fn run_sse(&self, bind: &str) -> Result<(), ErrorData> {
        let server = self.build_server();
        let listener = TcpListener::bind(bind).await
            .map_err(|e| ErrorData::internal_error(e.to_string()))?;
        server.run_sse(listener).await
    }

    fn build_server(&self) -> Server {
        let mut server = Server::new("chaotic-semantic-memory")
            .version(env!("CARGO_PKG_VERSION"))
            .with_tool_handler(MemoryTools { framework: self.framework.clone() })
            .with_resource_handler(MemoryResources { framework: self.framework.clone() });

        // Register tools with schemas
        server = server
            .tool(Tool::new("memory_inject", "Store a concept with vector and metadata", MemoryInjectInput::json_schema()))
            .tool(Tool::new("memory_inject_text", "Store a concept from text", MemoryInjectTextInput::json_schema()))
            .tool(Tool::new("memory_probe", "Top-K similarity search with vector", MemoryProbeInput::json_schema()))
            .tool(Tool::new("memory_probe_text", "Top-K similarity search with text query", MemoryProbeTextInput::json_schema()))
            .tool(Tool::new("memory_probe_filtered", "Filtered similarity search with text query", MemoryProbeFilteredInput::json_schema()))
            .tool(Tool::new("memory_get", "Fetch concept by ID", MemoryGetInput::json_schema()))
            .tool(Tool::new("memory_delete", "Remove a concept", MemoryDeleteInput::json_schema()))
            .tool(Tool::new("memory_associate", "Create directed association", MemoryAssociateInput::json_schema()))
            .tool(Tool::new("memory_traverse", "BFS from concept", MemoryTraverseInput::json_schema()))
            .tool(Tool::new("memory_shortest_path", "Path between concepts", MemoryShortestPathInput::json_schema()))
            .tool(Tool::new("memory_stats", "DB stats snapshot", MemoryStatsInput::json_schema()))
            .tool(Tool::new("memory_export", "Export to JSON", MemoryExportInput::json_schema()));

        server
    }
}

/// Helper trait to simplify tool registration with JSON Schema
trait JsonSchemaExt {
    fn json_schema() -> serde_json::Value;
}

impl<T: rmcp::schemars::JsonSchema> JsonSchemaExt for T {
    fn json_schema() -> serde_json::Value {
        let mut settings = rmcp::schemars::r#gen::SchemaSettings::draft07();
        settings.inline_subschemas = true;
        let generator = settings.into_generator();
        let schema = generator.into_root_schema_for::<T>();
        serde_json::to_value(schema).unwrap_or(serde_json::Value::Null)
    }
}
