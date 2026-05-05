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
        // Convert to f32 vector for API consistency.
        let hv = self.encoder.encode(text);
        Ok(hv.data.iter().map(|b| *b as f32).collect())
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // HDC encoding is fast, batch is just iteration.
        let mut results = Vec::with_capacity(texts.len());
        for t in texts {
            results.push(self.embed(t).await?);
        }
        Ok(results)
    }

    fn project(&self, vec: &[f32], _projection: &Projection) -> HVec10240 {
        // HDC provider outputs 10240-dim f32; convert back to HVec.
        // No projection matrix needed since native_dim == 10240.
        // bit = 1 if value > 0.5, bit = 0 otherwise
        let mut hv = HVec10240::zero();
        for (i, &v) in vec.iter().take(10240).enumerate() {
            if v > 0.5 {
                let word = i / 128;
                let bit = i % 128;
                hv.data[word] |= 1u128 << bit;
            }
        }
        hv
    }
}
