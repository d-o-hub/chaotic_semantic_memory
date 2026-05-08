//! OpenAI embeddings API backend.
//!
//! Requires `embed-openai` feature. Uses text-embedding-3-small by default.

use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};
use serde::Deserialize;

/// OpenAI embedding provider via HTTP API.
///
/// Default model: text-embedding-3-small (1536 dimensions).
/// API key must be set via environment or constructor.
#[derive(Debug)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    /// Create with API key from environment (OPENAI_API_KEY).
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| MemoryError::Config("OPENAI_API_KEY not set".into()))?;
        Self::new(api_key)
    }

    /// Create with explicit API key.
    pub fn new(api_key: String) -> Result<Self> {
        Ok(Self {
            api_key,
            model: "text-embedding-3-small".into(),
            base_url: "https://api.openai.com/v1".into(),
        })
    }

    /// Override model (e.g., text-embedding-3-large).
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override base URL (for proxies/azure).
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAiProvider {
    fn dimension(&self) -> usize { 1536 }
    fn name(&self) -> &str {
        "openai"
    }

    fn native_dim(&self) -> usize {
        // text-embedding-3-small: 1536
        // text-embedding-3-large: 3072
        if self.model.contains("large") {
            3072
        } else {
            1536
        }
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "embed-openai")]
        {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&serde_json::json!({
                    "input": text,
                    "model": self.model
                }))
                .send()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            let data: OpenAiResponse = response
                .json()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            data.data
                .first()
                .map(|d| d.embedding.clone())
                .ok_or_else(|| MemoryError::External("no embedding returned".into()))
        }

        #[cfg(not(feature = "embed-openai"))]
        {
            Err(MemoryError::Config(
                "embed-openai feature not enabled".into(),
            ))
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        #[cfg(feature = "embed-openai")]
        {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&serde_json::json!({
                    "input": texts,
                    "model": self.model
                }))
                .send()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            let data: OpenAiResponse = response
                .json()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            Ok(data.data.into_iter().map(|d| d.embedding).collect())
        }

        #[cfg(not(feature = "embed-openai"))]
        {
            Err(MemoryError::Config(
                "embed-openai feature not enabled".into(),
            ))
        }
    }
}

/// OpenAI API response structure.
#[cfg(feature = "embed-openai")]
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    data: Vec<OpenAiEmbedding>,
}

#[cfg(feature = "embed-openai")]
#[derive(Debug, Deserialize)]
struct OpenAiEmbedding {
    embedding: Vec<f32>,
}
