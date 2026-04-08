use anyhow::Result;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use chaotic_semantic_memory::retrieval::hybrid::{compute_weights, merge_results};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Tokenize text for BM25 matching, stripping punctuation.
fn tokenize_for_bm25(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|s| !s.is_empty() && s.len() > 1) // Skip single chars
        .map(|s| s.to_string())
        .collect()
}

pub struct MemoryAdapter {
    framework: ChaoticSemanticFramework,
    bm25_index: RwLock<Bm25Index>,
    text_store: RwLock<HashMap<String, String>>,
}

impl MemoryAdapter {
    pub async fn new_in_memory() -> Result<Self> {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await?;
        Ok(Self {
            framework,
            bm25_index: RwLock::new(Bm25Index::new()),
            text_store: RwLock::new(HashMap::new()),
        })
    }

    pub async fn ingest_memory(&self, id: &str, text: &str) -> Result<()> {
        // Store text metadata for HDC
        let mut metadata = HashMap::new();
        metadata.insert("_text".to_string(), serde_json::Value::String(text.to_string()));
        self.framework.inject_text_with_metadata(id, text, metadata).await?;

        // Tokenize and add to BM25 index
        let tokens = tokenize_for_bm25(text);
        self.bm25_index.write().await.add_document(id, &tokens);

        // Store text for retrieval
        self.text_store.write().await.insert(id.to_string(), text.to_string());

        Ok(())
    }

    pub async fn query(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        // Get HDC results
        let hdc_hits = self.framework.probe_text(text, top_k * 3).await?; // Get more for filtering

        // Get BM25 results
        let query_tokens = tokenize_for_bm25(text);
        let bm25_hits = self.bm25_index.read().await.search(&query_tokens, top_k * 3);

        // Compute weights based on query length
        let weights = compute_weights(query_tokens.len());

        // Filter HDC results below threshold to avoid noise
        const HDC_MIN_SCORE: f32 = 0.15;
        let hdc_filtered: Vec<_> = hdc_hits
            .into_iter()
            .filter(|(_, score)| *score >= HDC_MIN_SCORE)
            .collect();

        // Merge results
        let merged = merge_results(&bm25_hits, &hdc_filtered, weights);

        // Return top_k
        Ok(merged.into_iter().take(top_k).collect())
    }

    /// Query with session filtering - only returns documents from the specified session.
    pub async fn query_in_session(&self, text: &str, session_id: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        // Get all results
        let all_results = self.query(text, top_k * 3).await?;

        // Filter to session-specific results
        let session_prefix = format!("{}:", session_id);
        let filtered: Vec<_> = all_results
            .into_iter()
            .filter(|(id, _)| id.starts_with(&session_prefix))
            .take(top_k)
            .collect();

        Ok(filtered)
    }

    pub async fn get_text(&self, id: &str) -> Result<Option<String>> {
        // Fast lookup from text store
        let text = self.text_store.read().await.get(id).cloned();
        if text.is_some() {
            return Ok(text);
        }

        // Fallback to framework concept
        let concept = self.framework.get_concept(id).await?;
        Ok(concept.map(|c| {
            c.metadata
                .get("_text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or(c.id)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_text_storage_retrieval() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();

        // Inject memory with text
        adapter.ingest_memory("test-1", "Hello world from memory").await.unwrap();

        // Retrieve the stored text
        let text = adapter.get_text("test-1").await.unwrap();
        assert_eq!(text, Some("Hello world from memory".to_string()));

        // Verify it's not just the ID
        assert_ne!(text, Some("test-1".to_string()));
    }

    #[tokio::test]
    async fn test_text_not_found() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();

        // Query for non-existent ID
        let text = adapter.get_text("nonexistent").await.unwrap();
        assert_eq!(text, None);
    }
}
