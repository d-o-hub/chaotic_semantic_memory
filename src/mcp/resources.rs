use crate::framework::ChaoticSemanticFramework;
use rmcp::model::{Resource, ResourceContents, ErrorData};
use std::sync::Arc;
use futures::Future;

pub struct MemoryResources {
    pub framework: Arc<ChaoticSemanticFramework>,
}

impl rmcp::ServerHandler for MemoryResources {
    // We don't really need a separate ResourceHandler if we implement everything in ServerHandler
}
