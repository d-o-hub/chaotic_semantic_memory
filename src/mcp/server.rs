use crate::framework::ChaoticSemanticFramework;
use crate::mcp::tools::MemoryHandler;
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
        let service = MemoryHandler {
            framework: self.framework.clone(),
        };
        let transport = rmcp::transport::stdio();
        let handle = rmcp::service::serve_server(service, transport)
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("Stdio server execution failed: {:?}", e), None)
            })?;

        handle.waiting().await.map_err(|e| {
            ErrorData::internal_error(format!("Stdio server failed: {:?}", e), None)
        })?;

        Ok(())
    }

    pub async fn run_sse(&self, bind: &str) -> Result<(), ErrorData> {
        let service_provider = {
            let framework = self.framework.clone();
            move || MemoryHandler {
                framework: framework.clone(),
            }
        };

        let addr: std::net::SocketAddr = bind.parse().map_err(|e: std::net::AddrParseError| {
            ErrorData::internal_error(e.to_string(), None)
        })?;

        let sse_server = rmcp::transport::sse_server::SseServer::serve(addr)
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        sse_server.with_service(service_provider).cancelled().await;

        Ok(())
    }
}
