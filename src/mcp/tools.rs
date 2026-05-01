use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
use crate::mcp::schema::*;
use rmcp::model::{ErrorData, CallToolResult, ServerInfo, ListToolsResult, ListResourcesResult, ReadResourceResult, Tool, Content, Implementation};
use std::sync::Arc;
use futures::{Future, FutureExt};

pub struct MemoryTools {
    pub framework: Arc<ChaoticSemanticFramework>,
}

impl rmcp::ServerHandler for MemoryTools {
    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        let name = request.name.as_ref().to_string();
        let arguments = request.arguments.map(serde_json::Value::Object).unwrap_or(serde_json::Value::Null);

        async move {
            match name.as_str() {
                "memory_inject" => {
                    let input: MemoryInjectInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let vector = HVec10240::from_bytes(&input.vector)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    self.framework.inject_concept_with_metadata(input.id, vector, input.metadata).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Concept injected successfully")]))
                }
                "memory_inject_text" => {
                    let input: MemoryInjectTextInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    self.framework.inject_text_with_metadata(&input.id, &input.text, input.metadata).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Text injected successfully")]))
                }
                "memory_probe" => {
                    let input: MemoryProbeInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let vector = HVec10240::from_bytes(&input.vector)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    let results = self.framework.probe(vector, input.top_k).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&results)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_probe_text" => {
                    let input: MemoryProbeTextInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let results = self.framework.probe_text(&input.query, input.top_k).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&results)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_probe_filtered" => {
                    let input: MemoryProbeFilteredInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let filter: MetadataFilter = serde_json::from_value(input.filter)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let results = self.framework.probe_filtered_text(&input.query, input.top_k, &filter).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&results)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_get" => {
                    let input: MemoryGetInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let concept = self.framework.get_concept(&input.id).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&concept)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_delete" => {
                    let input: MemoryDeleteInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    self.framework.delete_concept(&input.id).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Concept deleted successfully")]))
                }
                "memory_associate" => {
                    let input: MemoryAssociateInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    self.framework.associate(&input.from, &input.to, input.strength).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Association created successfully")]))
                }
                "memory_traverse" => {
                    let input: MemoryTraverseInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let config = crate::graph_traversal::TraversalConfig {
                        max_depth: input.max_depth as usize,
                        min_strength: input.min_strength,
                        max_results: 1000,
                    };
                    let results = self.framework.traverse(&input.start_id, config).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&results)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_shortest_path" => {
                    let input: MemoryShortestPathInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    let path = self.framework.shortest_path(&input.from, &input.to).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&path)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_stats" => {
                    let stats = self.framework.stats().await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&stats)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?)]))
                }
                "memory_export" => {
                    let input: MemoryExportInput = serde_json::from_value(arguments)
                        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                    self.framework.export_json(&input.path).await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(format!("Memory exported to {}", input.path))]))
                }
                _ => Err(ErrorData::method_not_found::<rmcp::model::CallToolRequestMethod>()),
            }
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        use rmcp::handler::server::tool::cached_schema_for_type;

        async move {
            Ok(ListToolsResult {
                tools: vec![
                    Tool::new("memory_inject", "Store a concept with vector and metadata", cached_schema_for_type::<MemoryInjectInput>()),
                    Tool::new("memory_inject_text", "Store a concept from text", cached_schema_for_type::<MemoryInjectTextInput>()),
                    Tool::new("memory_probe", "Top-K similarity search with vector", cached_schema_for_type::<MemoryProbeInput>()),
                    Tool::new("memory_probe_text", "Top-K similarity search with text query", cached_schema_for_type::<MemoryProbeTextInput>()),
                    Tool::new("memory_probe_filtered", "Filtered similarity search with text query", cached_schema_for_type::<MemoryProbeFilteredInput>()),
                    Tool::new("memory_get", "Fetch concept by ID", cached_schema_for_type::<MemoryGetInput>()),
                    Tool::new("memory_delete", "Remove a concept", cached_schema_for_type::<MemoryDeleteInput>()),
                    Tool::new("memory_associate", "Create directed association", cached_schema_for_type::<MemoryAssociateInput>()),
                    Tool::new("memory_traverse", "BFS from concept", cached_schema_for_type::<MemoryTraverseInput>()),
                    Tool::new("memory_shortest_path", "Path between concepts", cached_schema_for_type::<MemoryShortestPathInput>()),
                    Tool::new("memory_stats", "DB stats snapshot", cached_schema_for_type::<MemoryStatsInput>()),
                    Tool::new("memory_export", "Export to JSON", cached_schema_for_type::<MemoryExportInput>()),
                ],
                next_cursor: None,
            })
        }
    }

    fn get_info(&self) -> rmcp::model::ServerInfo {
        Implementation {
            name: "chaotic-semantic-memory".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async move {
            Ok(ListResourcesResult {
                resources: vec![],
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        _request: rmcp::model::ReadResourceRequestParam,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_ {
        async move {
            Ok(ReadResourceResult {
                contents: vec![],
            })
        }
    }
}

// Add probe_filtered_text to framework since it wasn't there but we need it for MCP convenience
impl ChaoticSemanticFramework {
    pub async fn probe_filtered_text(&self, query: &str, top_k: usize, filter: &MetadataFilter) -> crate::error::Result<Vec<(String, f32)>> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(query);
        self.probe_filtered(&vector, top_k, filter).await
    }
}
