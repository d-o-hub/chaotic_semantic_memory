//! External embedding model bridge for semantic accuracy.
//!
//! Provides an `EmbeddingProvider` trait with multiple backends:
//! - HDC text encoder (default, semantically blind but fast)
//! - FastEmbed (local ONNX models, opt-in)
//! - OpenAI HTTP API (opt-in)
//! - Voyage HTTP API (opt-in)
//!
//! All backends project native embeddings to HVec10240 via sparse random projection
//! (Achlioptas method), preserving cosine similarity with Johnson-Lindenstrauss guarantees.

mod hdc_text;
mod projection;

pub use hdc_text::HdcTextProvider;
pub use projection::{Projection, ProjectionConfig};

use crate::error::Result;
use crate::hyperdim::HVec10240;

/// Embedding provider trait for text-to-vector conversion.
///
/// Implementations may be:
/// - Local (HDC hash, FastEmbed ONNX)
/// - Remote HTTP (OpenAI, Voyage)
///
/// All providers project their native dimensionality to HVec10240.
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider name for logging and CLI.
    fn name(&self) -> &str;

    /// Native embedding dimension before projection.
    fn native_dim(&self) -> usize;

    /// Embed a single text string.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed multiple texts in batch (more efficient for remote providers).
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Project a native embedding to HVec10240.
    ///
    /// Default implementation uses the provider's projection matrix.
    fn project(&self, vec: &[f32], projection: &Projection) -> HVec10240 {
        projection.project(vec)
    }
}
