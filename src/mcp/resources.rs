use crate::framework::ChaoticSemanticFramework;
use rmcp::model::{
    Annotated, ErrorData, RawResource, RawResourceTemplate, ReadResourceResult, Resource,
    ResourceContents, ResourceTemplate,
};

pub async fn list_resources() -> Vec<Resource> {
    vec![
        Annotated::new(
            RawResource {
                uri: "stats://current".to_string(),
                name: "Live framework statistics".to_string(),
                description: Some("Live snapshot of reservoir and persistence metrics".to_string()),
                mime_type: Some("application/json".to_string()),
                size: None,
            },
            None,
        ),
        Annotated::new(
            RawResource {
                uri: "health://current".to_string(),
                name: "Persistence health check status".to_string(),
                description: Some("Current status of the underlying database and WAL".to_string()),
                mime_type: Some("application/json".to_string()),
                size: None,
            },
            None,
        ),
    ]
}

pub async fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![Annotated::new(
        RawResourceTemplate {
            uri_template: "concept://{id}".to_string(),
            name: "Concept Details".to_string(),
            description: Some("JSON representation of a single concept by ID".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        None,
    )]
}

pub async fn read_resource(
    framework: &ChaoticSemanticFramework,
    uri: &str,
) -> Result<ReadResourceResult, ErrorData> {
    if uri == "stats://current" {
        let stats = framework
            .stats()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let content = serde_json::to_string_pretty(&stats)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        return Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, uri)],
        });
    }

    if uri == "health://current" {
        framework
            .persistence_health_check()
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let content = serde_json::to_string_pretty(&())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        return Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, uri)],
        });
    }

    if let Some(id) = uri.strip_prefix("concept://") {
        let concept = framework
            .get_concept(id)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        if let Some(c) = concept {
            let content = serde_json::to_string_pretty(&c)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, uri)],
            });
        } else {
            return Err(ErrorData::invalid_params(
                format!("Concept not found: {}", id),
                None,
            ));
        }
    }

    Err(ErrorData::invalid_params(
        format!("Unsupported URI: {}", uri),
        None,
    ))
}
