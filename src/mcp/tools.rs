//! MCP Tool handlers (ADR-0067)
//!
//! 12 tools mapping CLI commands to MCP tool calls.

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::OnceCell;
use tracing::info;

use crate::framework::ChaoticSemanticFramework;

use super::schema;

/// MCP Tools handler for chaotic_semantic_memory operations.
pub struct McpTools {
    database: Option<PathBuf>,
    framework: OnceCell<ChaoticSemanticFramework>,
}

impl McpTools {
    /// Create new MCP tools handler.
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
                info!("Initializing ChaoticSemanticFramework");
                match crate::cli::commands::create_framework(self.database.as_deref()).await {
                    Ok(fw) => {
                        info!("ChaoticSemanticFramework initialized");
                        Ok(fw)
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to initialize framework: {e}");
                        tracing::error!("{}", err_msg);
                        Err(anyhow::anyhow!(err_msg))
                    }
                }
            })
            .await
    }

    /// List all available tools.
    pub fn list_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "memory_inject".to_string(),
                description: "Store a concept with optional metadata".to_string(),
                input_schema: schema::inject_schema(),
            },
            ToolDefinition {
                name: "memory_inject_text".to_string(),
                description: "Store a concept from text (uses TextEncoder)".to_string(),
                input_schema: schema::inject_text_schema(),
            },
            ToolDefinition {
                name: "memory_probe".to_string(),
                description: "Top-K similarity search by vector ID".to_string(),
                input_schema: schema::probe_schema(),
            },
            ToolDefinition {
                name: "memory_probe_text".to_string(),
                description: "Text-query similarity search".to_string(),
                input_schema: schema::probe_text_schema(),
            },
            ToolDefinition {
                name: "memory_probe_filtered".to_string(),
                description: "Metadata-filtered similarity search".to_string(),
                input_schema: schema::probe_filtered_schema(),
            },
            ToolDefinition {
                name: "memory_get".to_string(),
                description: "Fetch concept by ID".to_string(),
                input_schema: schema::get_schema(),
            },
            ToolDefinition {
                name: "memory_delete".to_string(),
                description: "Remove a concept".to_string(),
                input_schema: schema::delete_schema(),
            },
            ToolDefinition {
                name: "memory_associate".to_string(),
                description: "Create directed association".to_string(),
                input_schema: schema::associate_schema(),
            },
            ToolDefinition {
                name: "memory_traverse".to_string(),
                description: "BFS graph traversal from concept".to_string(),
                input_schema: schema::traverse_schema(),
            },
            ToolDefinition {
                name: "memory_shortest_path".to_string(),
                description: "Find path between concepts".to_string(),
                input_schema: schema::shortest_path_schema(),
            },
            ToolDefinition {
                name: "memory_stats".to_string(),
                description: "DB stats snapshot".to_string(),
                input_schema: schema::stats_schema(),
            },
            ToolDefinition {
                name: "memory_export".to_string(),
                description: "Export to JSON".to_string(),
                input_schema: schema::export_schema(),
            },
        ]
    }

    /// Execute a tool call.
    ///
    /// # Errors
    ///
    /// Returns error if tool execution fails.
    pub async fn execute(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        info!(
            "Executing MCP tool: {} with args: {:?}",
            tool_name, arguments
        );

        match tool_name {
            "memory_inject" => self.handle_inject(arguments).await,
            "memory_inject_text" => self.handle_inject_text(arguments).await,
            "memory_probe" => self.handle_probe(arguments).await,
            "memory_probe_text" => self.handle_probe_text(arguments).await,
            "memory_probe_filtered" => self.handle_probe_filtered(arguments).await,
            "memory_get" => self.handle_get(arguments).await,
            "memory_delete" => self.handle_delete(arguments).await,
            "memory_associate" => self.handle_associate(arguments).await,
            "memory_traverse" => self.handle_traverse(arguments).await,
            "memory_shortest_path" => self.handle_shortest_path(arguments).await,
            "memory_stats" => self.handle_stats(arguments).await,
            "memory_export" => self.handle_export(arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {tool_name}")),
        }
    }

    async fn handle_inject(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.inject_concept
        Ok(serde_json::json!({"status": "ok", "message": "inject stub"}))
    }

    async fn handle_inject_text(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.inject_text
        Ok(serde_json::json!({"status": "ok", "message": "inject_text stub"}))
    }

    async fn handle_probe(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.probe
        Ok(serde_json::json!({"status": "ok", "results": []}))
    }

    async fn handle_probe_text(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.probe_text
        Ok(serde_json::json!({"status": "ok", "results": []}))
    }

    async fn handle_probe_filtered(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.probe_filtered
        Ok(serde_json::json!({"status": "ok", "results": []}))
    }

    async fn handle_get(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.get_concept
        Ok(serde_json::json!({"status": "ok", "concept": null}))
    }

    async fn handle_delete(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.delete_concept
        Ok(serde_json::json!({"status": "ok", "deleted": false}))
    }

    async fn handle_associate(&self, args: Value) -> Result<Value> {
        let from_id = args["from_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing from_id"))?;
        let to_id = args["to_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing to_id"))?;
        let strength = args["strength"].as_f64().unwrap_or(0.5);

        let framework = self.framework().await?;
        framework.associate(from_id, to_id, strength as f32).await?;

        Ok(serde_json::json!({
            "status": "ok",
            "from_id": from_id,
            "to_id": to_id,
            "strength": strength
        }))
    }

    async fn handle_traverse(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.traverse
        Ok(serde_json::json!({"status": "ok", "nodes": []}))
    }

    async fn handle_shortest_path(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.shortest_path
        Ok(serde_json::json!({"status": "ok", "path": []}))
    }

    async fn handle_stats(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.stats
        Ok(serde_json::json!({"status": "ok", "concepts": 0, "associations": 0}))
    }

    async fn handle_export(&self, _args: Value) -> Result<Value> {
        // TODO: Wire to framework.export
        Ok(serde_json::json!({"status": "ok", "data": []}))
    }
}

/// Tool definition for MCP tools/list response.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input validation
    pub input_schema: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_handle_associate() -> Result<()> {
        let tools = McpTools::new(None);

        // Inject concepts first
        {
            let framework = tools.framework().await?;
            framework
                .inject_concept("concept-a", crate::hyperdim::HVec10240::random())
                .await?;
            framework
                .inject_concept("concept-b", crate::hyperdim::HVec10240::random())
                .await?;
        }

        let args = json!({
            "from_id": "concept-a",
            "to_id": "concept-b",
            "strength": 0.8
        });

        let response = tools.handle_associate(args).await?;

        assert_eq!(response["status"], "ok");
        assert_eq!(response["from_id"], "concept-a");
        assert_eq!(response["to_id"], "concept-b");
        assert_eq!(response["strength"], 0.8);

        // Verify persistence
        let framework = tools.framework().await?;
        let associations = framework.get_associations("concept-a").await?;
        let found = associations
            .iter()
            .any(|(id, strength)| id == "concept-b" && (*strength - 0.8).abs() < 1e-6);
        assert!(found, "Association not found in framework");

        Ok(())
    }

    #[tokio::test]
    async fn test_handle_associate_default_strength() -> Result<()> {
        let tools = McpTools::new(None);

        // Inject concepts first
        {
            let framework = tools.framework().await?;
            framework
                .inject_concept("concept-a", crate::hyperdim::HVec10240::random())
                .await?;
            framework
                .inject_concept("concept-b", crate::hyperdim::HVec10240::random())
                .await?;
        }

        let args = json!({
            "from_id": "concept-a",
            "to_id": "concept-b"
        });

        let response = tools.handle_associate(args).await?;

        assert_eq!(response["status"], "ok");
        assert_eq!(response["strength"], 0.5);

        Ok(())
    }

    #[tokio::test]
    async fn test_handle_associate_missing_args() {
        let tools = McpTools::new(None);
        let args = json!({
            "from_id": "concept-a"
        });

        let result = tools.handle_associate(args).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Missing to_id");
    }
}
