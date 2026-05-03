//! External embedding model bridge for semantic accuracy.
//!
//! Provides an `EmbeddingProvider` trait with multiple backends:
//! - HDC text encoder (default, semantically blind but fast)
//! - FastEmbed (local ONNX models, opt-in via `embed-fastembed` feature)
//! - OpenAI HTTP API (opt-in via `embed-openai` feature)
//! - Voyage HTTP API (opt-in via `embed-voyage` feature)
//!
//! All backends project native embeddings to HVec10240 via sparse random projection
//! (Achlioptas method), preserving cosine similarity with Johnson-Lindenstrauss guarantees.

mod hdc_text;
mod projection;

#[cfg(feature = "embed-fastembed")]
mod fastembed;
#[cfg(feature = "embed-openai")]
mod remote_openai;
#[cfg(feature = "embed-voyage")]
mod remote_voyage;

pub use hdc_text::HdcTextProvider;
pub use projection::{Projection, ProjectionConfig};

#[cfg(feature = "embed-fastembed")]
pub use fastembed::FastEmbedProvider;
#[cfg(feature = "embed-openai")]
pub use remote_openai::OpenAiProvider;
#[cfg(feature = "embed-voyage")]
pub use remote_voyage::VoyageProvider;

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
