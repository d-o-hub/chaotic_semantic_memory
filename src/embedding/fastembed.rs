//! FastEmbed local ONNX embeddings backend.
//!
//! Requires `embed-fastembed` feature. Runs models locally without API calls.
//! Default model: BGE-small-en-v1.5 (384 dimensions).

use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};

/// FastEmbed local embedding provider.
///
/// Uses ONNX runtime with downloaded models. No API key required.
/// Models are cached locally after first download.
///
/// Note: Does not implement Debug because fastembed::TextEmbedding doesn't.
pub struct FastEmbedProvider {
    #[cfg(feature = "embed-fastembed")]
    model: std::sync::Arc<fastembed::TextEmbedding>,
    model_name: String,
}

impl FastEmbedProvider {
    /// Create with default model (BGE-small-en-v1.5).
    pub fn new() -> Result<Self> {
        #[cfg(feature = "embed-fastembed")]
        {
            let model = fastembed::TextEmbedding::try_new(
                fastembed::InitOptions::new(fastembed::EmbeddingModel::BGESmallENV15)
                    .with_cache_dir(std::path::PathBuf::from(".fastembed_cache")),
            )
            .map_err(|e| MemoryError::External(format!("fastembed init failed: {e}")))?;

            Ok(Self {
                model: std::sync::Arc::new(model),
                model_name: "bge-small-en-v1.5".into(),
            })
        }

        #[cfg(not(feature = "embed-fastembed"))]
        {
            Err(MemoryError::Config(
                "embed-fastembed feature not enabled".into(),
            ))
        }
    }

    /// Create with specific model.
    pub fn with_model(model_name: &str) -> Result<Self> {
        #[cfg(feature = "embed-fastembed")]
        {
            let model_type = match model_name {
                "bge-small-en-v1.5" => fastembed::EmbeddingModel::BGESmallENV15,
                "bge-base-en-v1.5" => fastembed::EmbeddingModel::BGEBaseENV15,
                "bge-large-en-v1.5" => fastembed::EmbeddingModel::BGELargeENV15,
                "all-minilm-l6-v2" => fastembed::EmbeddingModel::AllMiniLML6V2,
                "nomic-embed-text-v1" => fastembed::EmbeddingModel::NomicEmbedTextV1,
                _ => {
                    return Err(MemoryError::Config(format!(
                        "unknown FastEmbed model: {model_name}"
                    )));
                }
            };

            let embedding = fastembed::TextEmbedding::try_new(
                fastembed::InitOptions::new(model_type)
                    .with_cache_dir(std::path::PathBuf::from(".fastembed_cache")),
            )
            .map_err(|e| MemoryError::External(format!("fastembed init failed: {e}")))?;

            Ok(Self {
                model: std::sync::Arc::new(embedding),
                model_name: model_name.into(),
            })
        }

        #[cfg(not(feature = "embed-fastembed"))]
        {
            Err(MemoryError::Config(
                "embed-fastembed feature not enabled".into(),
            ))
        }
    }

    /// Get the model name.
    #[must_use]
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for FastEmbedProvider {
    fn dimension(&self) -> usize { 384 }
    fn name(&self) -> &str {
        "fastembed"
    }

    fn native_dim(&self) -> usize {
        // Model dimensions:
        // - BGE-small: 384
        // - all-minilm-l6-v2: 384
        // - BGE-large: 1024
        // - BGE-base: 768
        // - nomic: 768
        if self.model_name.contains("small") || self.model_name.contains("minilm") {
            384
        } else if self.model_name.contains("large") {
            1024
        } else {
            768 // base models + nomic
        }
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "embed-fastembed")]
        {
            // FastEmbed is synchronous, wrap in blocking task
            let text_owned = text.to_owned();
            let model = self.model.clone();

            let result: Result<Vec<f32>> = tokio::task::spawn_blocking(move || {
                let embeddings = model
                    .embed(vec![text_owned], None)
                    .map_err(|e| MemoryError::External(format!("fastembed embed failed: {e}")))?;

                embeddings
                    .first()
                    .cloned()
                    .ok_or_else(|| MemoryError::External("no embedding returned".into()))
            })
            .await
            .map_err(|e| MemoryError::External(format!("blocking task failed: {e}")))?;

            result
        }

        #[cfg(not(feature = "embed-fastembed"))]
        {
            Err(MemoryError::Config(
                "embed-fastembed feature not enabled".into(),
            ))
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        #[cfg(feature = "embed-fastembed")]
        {
            let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
            let model = self.model.clone();

            let result: Result<Vec<Vec<f32>>> = tokio::task::spawn_blocking(move || {
                model.embed(texts_owned, None).map_err(|e| {
                    MemoryError::External(format!("fastembed embed_batch failed: {e}"))
                })
            })
            .await
            .map_err(|e| MemoryError::External(format!("blocking task failed: {e}")))?;

            result
        }

        #[cfg(not(feature = "embed-fastembed"))]
        {
            Err(MemoryError::Config(
                "embed-fastembed feature not enabled".into(),
            ))
        }
    }
}
