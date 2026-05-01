use crate::framework::ChaoticSemanticFramework;
use crate::mcp::tools::MemoryTools;
use rmcp::model::ErrorData;
use std::sync::Arc;

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
        let service = MemoryTools { framework: self.framework.clone() };
        let transport = rmcp::transport::stdio();
        rmcp::service::serve_server(service, transport).await
            .map_err(|_| ErrorData::internal_error("Stdio server execution failed", None))?;
        Ok(())
    }

    pub async fn run_sse(&self, bind: &str) -> Result<(), ErrorData> {
        let service_provider = {
            let framework = self.framework.clone();
            move || MemoryTools { framework: framework.clone() }
        };

        let config = rmcp::transport::sse_server::SseServerConfig {
            bind: bind.parse().map_err(|e: std::net::AddrParseError| ErrorData::internal_error(e.to_string(), None))?,
            ..Default::default()
        };

        let sse_server = rmcp::transport::sse_server::SseServer::serve(config).await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        sse_server.with_service(service_provider).cancelled().await;

        Ok(())
    }
}
