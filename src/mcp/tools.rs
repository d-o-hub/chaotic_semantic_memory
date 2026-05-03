use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;
use crate::mcp::schema::*;
use crate::metadata_filter::MetadataFilter;
use rmcp::model::{
    CallToolResult, Content, ErrorData, Implementation, InitializeResult,
    ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, ProtocolVersion,
    ReadResourceResult, ServerCapabilities, Tool,
};
use std::sync::Arc;

pub struct MemoryHandler {
    pub framework: Arc<ChaoticSemanticFramework>,
}

impl MemoryHandler {
    async fn handle_memory_inject(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryInjectInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let vector = HVec10240::from_bytes(&input.vector)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        self.framework
            .inject_concept_with_metadata(input.id, vector, input.metadata)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Concept injected successfully",
        )]))
    }

    async fn handle_memory_inject_text(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryInjectTextInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        self.framework
            .inject_text_with_metadata(&input.id, &input.text, input.metadata)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Text injected successfully",
        )]))
    }

    async fn handle_memory_probe(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryProbeInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let vector = HVec10240::from_bytes(&input.vector)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let results = self
            .framework
            .probe(vector, input.top_k)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&results)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_probe_text(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryProbeTextInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let results = self
            .framework
            .probe_text(&input.query, input.top_k)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&results)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_probe_filtered(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryProbeFilteredInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let results = self
            .framework
            .probe_filtered_text(&input.query, input.top_k, &input.filter)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&results)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_get(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryGetInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let concept = self
            .framework
            .get_concept(&input.id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&concept)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_delete(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryDeleteInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        self.framework
            .delete_concept(&input.id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Concept deleted successfully",
        )]))
    }

    async fn handle_memory_associate(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryAssociateInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        self.framework
            .associate(&input.from, &input.to, input.strength)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Association created successfully",
        )]))
    }

    async fn handle_memory_traverse(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryTraverseInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let config = crate::graph_traversal::TraversalConfig {
            max_depth: input.max_depth as usize,
            min_strength: input.min_strength,
            max_results: 1000,
        };
        let results = self
            .framework
            .traverse(&input.start_id, config)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&results)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_shortest_path(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryShortestPathInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        let path = self
            .framework
            .shortest_path(&input.from, &input.to)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&path)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_stats(
        &self,
        _arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let stats = self
            .framework
            .stats()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&stats)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?,
        )]))
    }

    async fn handle_memory_export(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        let input: MemoryExportInput = serde_json::from_value(arguments)
            .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
        self.framework
            .export_json(&input.path)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Memory exported to {}",
            input.path
        ))]))
    }
}

impl rmcp::ServerHandler for MemoryHandler {
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let name = request.name.as_ref().to_string();
        let arguments = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::default()));

        match name.as_str() {
            "memory_inject" => self.handle_memory_inject(arguments).await,
            "memory_inject_text" => self.handle_memory_inject_text(arguments).await,
            "memory_probe" => self.handle_memory_probe(arguments).await,
            "memory_probe_text" => self.handle_memory_probe_text(arguments).await,
            "memory_probe_filtered" => self.handle_memory_probe_filtered(arguments).await,
            "memory_get" => self.handle_memory_get(arguments).await,
            "memory_delete" => self.handle_memory_delete(arguments).await,
            "memory_associate" => self.handle_memory_associate(arguments).await,
            "memory_traverse" => self.handle_memory_traverse(arguments).await,
            "memory_shortest_path" => self.handle_memory_shortest_path(arguments).await,
            "memory_stats" => self.handle_memory_stats(arguments).await,
            "memory_export" => self.handle_memory_export(arguments).await,
            _ => Err(ErrorData::method_not_found::<
                rmcp::model::CallToolRequestMethod,
            >()),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        use rmcp::handler::server::tool::cached_schema_for_type;

        Ok(ListToolsResult {
            tools: vec![
                Tool::new(
                    "memory_inject",
                    "Store a concept with vector and metadata",
                    cached_schema_for_type::<MemoryInjectInput>(),
                ),
                Tool::new(
                    "memory_inject_text",
                    "Store a concept from text",
                    cached_schema_for_type::<MemoryInjectTextInput>(),
                ),
                Tool::new(
                    "memory_probe",
                    "Top-K similarity search with vector",
                    cached_schema_for_type::<MemoryProbeInput>(),
                ),
                Tool::new(
                    "memory_probe_text",
                    "Top-K similarity search with text query",
                    cached_schema_for_type::<MemoryProbeTextInput>(),
                ),
                Tool::new(
                    "memory_probe_filtered",
                    "Filtered similarity search with text query",
                    cached_schema_for_type::<MemoryProbeFilteredInput>(),
                ),
                Tool::new(
                    "memory_get",
                    "Fetch concept by ID",
                    cached_schema_for_type::<MemoryGetInput>(),
                ),
                Tool::new(
                    "memory_delete",
                    "Remove a concept",
                    cached_schema_for_type::<MemoryDeleteInput>(),
                ),
                Tool::new(
                    "memory_associate",
                    "Create directed association",
                    cached_schema_for_type::<MemoryAssociateInput>(),
                ),
                Tool::new(
                    "memory_traverse",
                    "BFS from concept",
                    cached_schema_for_type::<MemoryTraverseInput>(),
                ),
                Tool::new(
                    "memory_shortest_path",
                    "Path between concepts",
                    cached_schema_for_type::<MemoryShortestPathInput>(),
                ),
                Tool::new(
                    "memory_stats",
                    "DB stats snapshot",
                    cached_schema_for_type::<MemoryStatsInput>(),
                ),
                Tool::new(
                    "memory_export",
                    "Export to JSON",
                    cached_schema_for_type::<MemoryExportInput>(),
                ),
            ],
            next_cursor: None,
        })
    }

    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "chaotic-semantic-memory".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: None,
        }
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: crate::mcp::resources::list_resources().await,
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            resource_templates: crate::mcp::resources::list_resource_templates().await,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        crate::mcp::resources::read_resource(&self.framework, request.uri.as_ref()).await
    }
}

// Add probe_filtered_text to framework since it wasn't there but we need it for MCP convenience
impl ChaoticSemanticFramework {
    pub async fn probe_filtered_text(
        &self,
        query: &str,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> crate::error::Result<Vec<(String, f32)>> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(query);
        self.probe_filtered(&vector, top_k, filter).await
    }
}
