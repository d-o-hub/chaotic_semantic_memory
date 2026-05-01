//! MCP Resources provider (ADR-0067)
//!
//! Provides concept://, stats://, and health:// resource URIs.

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

/// MCP Resources handler for chaotic_semantic_memory.
pub struct McpResources {
    database: Option<PathBuf>,
}

impl McpResources {
    /// Create new MCP resources handler.
    pub const fn new(database: Option<PathBuf>) -> Self {
        Self { database }
    }

    /// List all available resources.
    pub fn list_resources() -> Vec<ResourceDefinition> {
        vec![
            ResourceDefinition {
                uri: "concept://{id}".to_string(),
                name: "Concept by ID".to_string(),
                description: "JSON serialization of one concept".to_string(),
                mime_type: "application/json".to_string(),
            },
            ResourceDefinition {
                uri: "stats://current".to_string(),
                name: "Current stats".to_string(),
                description: "Live framework stats snapshot".to_string(),
                mime_type: "application/json".to_string(),
            },
            ResourceDefinition {
                uri: "health://current".to_string(),
                name: "Health check".to_string(),
                description: "Persistence health status".to_string(),
                mime_type: "application/json".to_string(),
            },
        ]
    }

    /// Read a resource by URI.
    ///
    /// # Errors
    ///
    /// Returns error if resource not found or read fails.
    pub async fn read(&self, uri: &str) -> Result<Value> {
        if let Some(id) = uri.strip_prefix("concept://") {
            self.read_concept(id).await
        } else if uri == "stats://current" {
            self.read_stats().await
        } else if uri == "health://current" {
            self.read_health().await
        } else {
            Err(anyhow::anyhow!("Unknown resource URI: {}", uri))
        }
    }

    async fn read_concept(&self, _id: &str) -> Result<Value> {
        // TODO: Wire to framework.get_concept
        Ok(serde_json::json!({"id": _id, "vector": null, "metadata": {}}))
    }

    async fn read_stats(&self) -> Result<Value> {
        // TODO: Wire to framework.stats
        Ok(serde_json::json!({"concepts": 0, "associations": 0, "cache_hits": 0}))
    }

    async fn read_health(&self) -> Result<Value> {
        // TODO: Wire to framework.persistence_health_check
        Ok(serde_json::json!({"status": "healthy", "database": self.database.is_some()}))
    }
}

/// Resource definition for MCP resources/list response.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceDefinition {
    /// Resource URI template
    pub uri: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// MIME type
    pub mime_type: String,
}
