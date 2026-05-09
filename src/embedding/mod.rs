//! Embedding provider abstraction and random projection (ADR-0069).

use crate::hyperdim::HVec10240;
pub mod projection;
pub mod remote_openai;
pub mod remote_voyage;

#[cfg(feature = "embed-fastembed")]
pub mod fastembed;
pub mod hdc_text;

use crate::error::Result;
pub use projection::Projection;
use std::sync::Arc;

/// Trait for external embedding models.
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Dimension of the source embeddings (e.g. 1536 for OpenAI)
    fn dimension(&self) -> usize;

    /// Provider name
    fn name(&self) -> &str;

    /// Embed text into high-dimensional float vector
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Project float embedding to 10k-bit hypervector
    fn project(&self, vec: &[f32], projection: &Projection) -> HVec10240 {
        projection.project(vec)
    }
}

/// Factory to get an embedding provider by name.
pub fn get_embedding_provider(
    name: &str,
    api_key: Option<String>,
) -> Result<Arc<dyn EmbeddingProvider>> {
    match name {
        "hdc" => Ok(Arc::new(hdc_text::HdcTextProvider::new())),
        "openai" => {
            let key = api_key.ok_or_else(|| crate::error::MemoryError::InvalidInput {
                field: "api_key".to_string(),
                reason: "OpenAI provider requires an API key".to_string(),
            })?;
            Ok(Arc::new(remote_openai::OpenAiProvider::new(key)?))
        }
        "voyage" => {
            let key = api_key.ok_or_else(|| crate::error::MemoryError::InvalidInput {
                field: "api_key".to_string(),
                reason: "Voyage provider requires an API key".to_string(),
            })?;
            Ok(Arc::new(remote_voyage::VoyageProvider::new(key)?))
        }
        #[cfg(feature = "embed-fastembed")]
        "fastembed" => Ok(Arc::new(fastembed::FastEmbedProvider::new()?)),
        _ => Err(crate::error::MemoryError::InvalidInput {
            field: "provider".to_string(),
            reason: format!("Unknown embedding provider: {name}"),
        }),
    }
}
