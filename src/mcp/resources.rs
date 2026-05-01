use crate::framework::ChaoticSemanticFramework;
use rmcp::model::{Resource, ResourceContents, ErrorData};
use std::sync::Arc;

pub struct MemoryResources {
    pub framework: Arc<ChaoticSemanticFramework>,
}

#[async_trait::async_trait]
impl rmcp::ResourceHandler for MemoryResources {
    async fn list(&self) -> Result<Vec<Resource>, ErrorData> {
        Ok(vec![
            Resource {
                uri: "stats://current".to_string(),
                name: "Current Memory Stats".to_string(),
                description: Some("Live framework statistics".to_string()),
                mime_type: Some("application/json".to_string()),
                ..Default::default()
            },
            Resource {
                uri: "health://current".to_string(),
                name: "Persistence Health".to_string(),
                description: Some("Persistence health check status".to_string()),
                mime_type: Some("application/json".to_string()),
                ..Default::default()
            },
            Resource {
                uri: "concept://{id}".to_string(),
                name: "Concept Details".to_string(),
                description: Some("JSON representation of a single concept".to_string()),
                mime_type: Some("application/json".to_string()),
                ..Default::default()
            },
        ])
    }

    async fn read(&self, uri: &str) -> Result<Vec<ResourceContents>, ErrorData> {
        if uri == "stats://current" {
            let stats = self.framework.stats().await
                .map_err(|e| ErrorData::internal_error(e.to_string()))?;
            return Ok(vec![ResourceContents::Text {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: serde_json::to_string_pretty(&stats)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?,
            }]);
        }

        if uri == "health://current" {
            let result = self.framework.persistence_health_check().await;
            let status = if result.is_ok() { "healthy" } else { "unhealthy" };
            let message = match result {
                Ok(_) => "OK".to_string(),
                Err(e) => e.to_string(),
            };
            return Ok(vec![ResourceContents::Text {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: serde_json::json!({
                    "status": status,
                    "message": message,
                }).to_string(),
            }]);
        }

        if let Some(id) = uri.strip_prefix("concept://") {
            let concept = self.framework.get_concept(id).await
                .map_err(|e| ErrorData::internal_error(e.to_string()))?;
            return Ok(vec![ResourceContents::Text {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: serde_json::to_string_pretty(&concept)
                    .map_err(|e| ErrorData::internal_error(e.to_string()))?,
            }]);
        }

        Err(ErrorData::internal_error(format!("Unknown resource: {}", uri)))
    }
}
