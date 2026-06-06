use csm_core::Result;

#[async_trait::async_trait]
pub trait Persistence: Send + Sync + std::fmt::Debug {
    async fn save_concept(&self, ns: &str, concept: &csm_memory::singularity::Concept) -> Result<()>;
    // ... more methods
}
