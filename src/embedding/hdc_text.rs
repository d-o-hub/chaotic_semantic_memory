#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! HDC text encoder as an embedding provider backend.
//!
//! Wraps the existing `TextEncoder` to implement `EmbeddingProvider`.
//! This is the default backend - semantically blind but deterministic and fast.

// Casts are intentional for HDC conversion (u128 words to f32 representation)

use crate::embedding::{EmbeddingProvider, Projection};
use crate::encoder::TextEncoder;
use crate::error::Result;
use crate::hyperdim::HVec10240;

/// HDC text embedding provider using FNV-1a hashing.
///
/// This is the default provider when no external embeddings are configured.
/// It generates hypervectors directly without an intermediate f32 embedding,
/// so the "native dimension" is 10240 and projection is identity.
#[derive(Debug, Clone)]
pub struct HdcTextProvider {
    encoder: TextEncoder,
}

impl Default for HdcTextProvider {
    fn default() -> Self {
        Self {
            encoder: TextEncoder::new(),
        }
    }
}

impl HdcTextProvider {
    /// Create a new HDC text provider with default encoder config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            encoder: TextEncoder::new(),
        }
    }

    /// Create with custom encoder configuration.
    #[must_use]
    pub const fn with_config(config: crate::encoder::TextEncoderConfig) -> Self {
        Self {
            encoder: TextEncoder::with_config(config),
        }
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for HdcTextProvider {
    fn name(&self) -> &str {
        "hdc-text"
    }

    fn native_dim(&self) -> usize {
        10240 // Direct to HVec, no projection needed
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // HDC encoder produces HVec10240 directly.
        // Preserve the full f32 values for lossless roundtrip.
        let hv = self.encoder.encode(text);
        Ok(hv.data.to_vec())
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // HDC encoding is fast, batch is just iteration.
        let mut results = Vec::with_capacity(texts.len());
        for t in texts {
            results.push(self.embed(t).await?);
        }
        Ok(results)
    }

    fn project(&self, vec: &[f32], projection: &Projection) -> HVec10240 {
        if projection.nnz() == 0 {
            // HDC provider outputs 10240-dim f32; preserve full values for roundtrip.
            // No projection matrix needed since native_dim == 10240.
            let mut data = [0.0; 10240];
            for (i, &v) in vec.iter().take(10240).enumerate() {
                data[i] = v;
            }
            HVec10240 { data }
        } else {
            projection.project(vec)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::Projection;
    use crate::hyperdim::Hypervector;

    #[tokio::test]
    async fn test_hdc_provider_roundtrip() {
        let provider = HdcTextProvider::new();
        let projection = Projection::empty();
        let text = "hello world";

        let embedding = provider.embed(text).await.unwrap();
        assert_eq!(embedding.len(), 10240);

        let vector = provider.project(&embedding, &projection);
        let zero = HVec10240::zero();
        assert_ne!(vector, zero, "HDC vector should not be zero");

        // Verify it matches direct encoding
        let direct = TextEncoder::new().encode(text);
        assert_eq!(vector, direct);
    }
}
