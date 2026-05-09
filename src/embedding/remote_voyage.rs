//! Voyage AI embeddings API backend.

use crate::embedding::EmbeddingProvider;
use crate::error::{MemoryError, Result};
#[cfg(feature = "embed-voyage")]
use serde::Deserialize;

#[derive(Debug)]
pub struct VoyageProvider {
    api_key: String,
    model: String,
}

impl VoyageProvider {
    pub fn new(api_key: String) -> Result<Self> {
        Ok(Self { api_key, model: "voyage-3".into() })
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for VoyageProvider {
    fn dimension(&self) -> usize { 1024 }
    fn name(&self) -> &str { "voyage" }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "embed-voyage")]
        {
            let client = reqwest::Client::new();
            let response = client
                .post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&serde_json::json!({ "input": text, "model": self.model }))
                .send()
                .await
                .map_err(|e| MemoryError::External(e.to_string()))?;

            let data: VoyageResponse = response.json().await.map_err(|e| MemoryError::External(e.to_string()))?;
            data.data.first().map(|d| d.embedding.clone()).ok_or_else(|| MemoryError::External("no embedding".into()))
        }
        #[cfg(not(feature = "embed-voyage"))]
        { let _ = text; Err(MemoryError::Config("embed-voyage disabled".into())) }
    }
}

#[cfg(feature = "embed-voyage")]
#[derive(Deserialize)]
struct VoyageResponse { data: Vec<VoyageEmbedding> }
#[cfg(feature = "embed-voyage")]
#[derive(Deserialize)]
struct VoyageEmbedding { embedding: Vec<f32> }
