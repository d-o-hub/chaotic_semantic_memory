//! FastEmbed (ONNX) local embedding backend.

use fastembed::TextEmbedding;
use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};
#[cfg(feature = "embed-fastembed")]

/// Local embedding provider using FastEmbed.
pub struct FastEmbedProvider {
    #[cfg(feature = "embed-fastembed")]
    model: TextEmbedding,
}

impl FastEmbedProvider {
    pub fn new() -> Result<Self> {
        #[cfg(feature = "embed-fastembed")]
        {
            let model = TextEmbedding::try_new(Default::default())
                .map_err(|e| MemoryError::External(e.to_string()))?;
            Ok(Self { model })
        }
        #[cfg(not(feature = "embed-fastembed"))]
        {
            Err(MemoryError::Config("embed-fastembed feature not enabled".into()))
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for FastEmbedProvider {
    fn dimension(&self) -> usize { 384 }
    fn name(&self) -> &str { "fastembed" }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "embed-fastembed")]
        {
            let embeddings = self.model.embed(vec![text], None)
                .map_err(|e| MemoryError::External(e.to_string()))?;
            Ok(embeddings[0].clone())
        }
        #[cfg(not(feature = "embed-fastembed"))]
        {
            let _ = text;
            Err(MemoryError::Config("embed-fastembed feature not enabled".into()))
        }
    }
}
