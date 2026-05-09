//! OpenAI embeddings API backend.

use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};
#[cfg(feature = "embed-openai")]
use serde::Deserialize;

#[derive(Debug)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Result<Self> {
        Ok(Self {
            api_key,
            model: "text-embedding-3-small".into(),
            base_url: "https://api.openai.com/v1".into(),
        })
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAiProvider {
    fn dimension(&self) -> usize {
        if self.model.contains("large") { 3072 } else { 1536 }
    }
    fn name(&self) -> &str { "openai" }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "embed-openai")]
        {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&serde_json::json!({ "input": text, "model": self.model }))
                .send()
                .await
                .map_err(|e| MemoryError::External(e.to_string()))?;

            let data: OpenAiResponse = response.json().await.map_err(|e| MemoryError::External(e.to_string()))?;
            data.data.first().map(|d| d.embedding.clone()).ok_or_else(|| MemoryError::External("no embedding".into()))
        }
        #[cfg(not(feature = "embed-openai"))]
        { let _ = text; Err(MemoryError::Config("embed-openai disabled".into())) }
    }
}

#[cfg(feature = "embed-openai")]
#[derive(Deserialize)]
struct OpenAiResponse { data: Vec<OpenAiEmbedding> }
#[cfg(feature = "embed-openai")]
#[derive(Deserialize)]
struct OpenAiEmbedding { embedding: Vec<f32> }
