use crate::framework::ChaoticSemanticFramework;

use std::sync::Arc;


pub struct MemoryResources {
    pub framework: Arc<ChaoticSemanticFramework>,
}

impl rmcp::ServerHandler for MemoryResources {
    // We don't really need a separate ResourceHandler if we implement everything in ServerHandler
}
