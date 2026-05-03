//! Voyage embeddings API backend.
//!
//! Requires `embed-voyage` feature. Uses voyage-2 by default.

use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};
use serde::Deserialize;

/// Voyage embedding provider via HTTP API.
///
/// Default model: voyage-2 (1024 dimensions).
/// API key must be set via environment or constructor.
#[derive(Debug)]
pub struct VoyageProvider {
    api_key: String,
    model: String,
}

impl VoyageProvider {
    /// Create with API key from environment (VOYAGE_API_KEY).
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("VOYAGE_API_KEY")
            .map_err(|_| MemoryError::Config("VOYAGE_API_KEY not set".into()))?;
        Self::new(api_key)
    }

    /// Create with explicit API key.
    pub fn new(api_key: String) -> Result<Self> {
        Ok(Self {
            api_key,
            model: "voyage-2".into(),
        })
    }

    /// Override model (e.g., voyage-3, voyage-code-2).
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for VoyageProvider {
    fn name(&self) -> &str {
        "voyage"
    }

    fn native_dim(&self) -> usize {
        // voyage-2/voyage-3: 1024
        // voyage-code-2: 1536
        if self.model.contains("code") {
            1536
        } else {
            1024
        }
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "embed-voyage")]
        {
            let client = reqwest::Client::new();
            let response = client
                .post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&serde_json::json!({
                    "input": [text],
                    "model": self.model
                }))
                .send()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            let data: VoyageResponse = response
                .json()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            data.data
                .first()
                .map(|d| d.embedding.clone())
                .ok_or_else(|| MemoryError::External("no embedding returned".into()))
        }

        #[cfg(not(feature = "embed-voyage"))]
        {
            Err(MemoryError::Config(
                "embed-voyage feature not enabled".into(),
            ))
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        #[cfg(feature = "embed-voyage")]
        {
            let client = reqwest::Client::new();
            let response = client
                .post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&serde_json::json!({
                    "input": texts,
                    "model": self.model
                }))
                .send()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            let data: VoyageResponse = response
                .json()
                .await
                .map_err(|e: reqwest::Error| MemoryError::External(e.to_string()))?;

            Ok(data.data.into_iter().map(|d| d.embedding).collect())
        }

        #[cfg(not(feature = "embed-voyage"))]
        {
            Err(MemoryError::Config(
                "embed-voyage feature not enabled".into(),
            ))
        }
    }
}

/// Voyage API response structure.
#[cfg(feature = "embed-voyage")]
#[derive(Debug, Deserialize)]
struct VoyageResponse {
    data: Vec<VoyageEmbedding>,
}

#[cfg(feature = "embed-voyage")]
#[derive(Debug, Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
}
