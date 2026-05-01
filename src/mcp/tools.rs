use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
use crate::mcp::schema::*;
use rmcp::model::{ErrorData, CallToolResult};
use std::sync::Arc;

pub struct MemoryTools {
    pub framework: Arc<ChaoticSemanticFramework>,
}

#[async_trait::async_trait]
impl rmcp::ToolHandler for MemoryTools {
    async fn call(&self, name: &str, arguments: serde_json::Value) -> Result<CallToolResult, ErrorData> {
        match name {
            "memory_inject" => {
                let input: MemoryInjectInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let vector = HVec10240::from_slice(&input.vector)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                self.framework.inject_concept_with_metadata(input.id, vector, input.metadata).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text("Concept injected successfully"))
            }
            "memory_inject_text" => {
                let input: MemoryInjectTextInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                self.framework.inject_text_with_metadata(&input.id, &input.text, input.metadata).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text("Text injected successfully"))
            }
            "memory_probe" => {
                let input: MemoryProbeInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let vector = HVec10240::from_slice(&input.vector)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                let results = self.framework.probe(vector, input.top_k).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&results)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_probe_text" => {
                let input: MemoryProbeTextInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let results = self.framework.probe_text(&input.query, input.top_k).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&results)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_probe_filtered" => {
                let input: MemoryProbeFilteredInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let filter: MetadataFilter = serde_json::from_value(input.filter)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let results = self.framework.probe_filtered_text(&input.query, input.top_k, &filter).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&results)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_get" => {
                let input: MemoryGetInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let concept = self.framework.get_concept(&input.id).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&concept)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_delete" => {
                let input: MemoryDeleteInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                self.framework.delete_concept(&input.id).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text("Concept deleted successfully"))
            }
            "memory_associate" => {
                let input: MemoryAssociateInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                self.framework.associate(&input.from, &input.to, input.strength).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text("Association created successfully"))
            }
            "memory_traverse" => {
                let input: MemoryTraverseInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let config = crate::graph_traversal::TraversalConfig {
                    max_depth: input.max_depth,
                    min_strength: input.min_strength,
                    ..Default::default()
                };
                let results = self.framework.traverse(&input.start_id, config).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&results)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_shortest_path" => {
                let input: MemoryShortestPathInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                let path = self.framework.shortest_path(&input.from, &input.to).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&path)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_stats" => {
                let stats = self.framework.stats().await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(serde_json::to_string_pretty(&stats)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?))
            }
            "memory_export" => {
                let input: MemoryExportInput = serde_json::from_value(arguments)
                    .map_err(|e| ErrorData::invalid_params(e.to_string()))?;
                self.framework.export_json(&input.path).await
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?;
                Ok(CallToolResult::new_text(format!("Memory exported to {}", input.path)))
            }
            _ => Err(ErrorData::method_not_found(name)),
        }
    }
}

// Add probe_filtered_text to framework since it wasn't there but we need it for MCP convenience
impl ChaoticSemanticFramework {
    pub async fn probe_filtered_text(&self, query: &str, top_k: usize, filter: &MetadataFilter) -> crate::error::Result<Vec<(String, f32)>> {
        let vector = self.encode_text(query).await?;
        self.probe_filtered(&vector, top_k, filter).await
    }

    async fn encode_text(&self, text: &str) -> crate::error::Result<HVec10240> {
        let encoder = crate::encoder::TextEncoder::new();
        Ok(encoder.encode(text))
    }
}
