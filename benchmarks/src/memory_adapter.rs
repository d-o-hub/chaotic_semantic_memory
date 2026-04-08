use anyhow::Result;
use chaotic_semantic_memory::prelude::*;
use std::collections::HashMap;

pub struct MemoryAdapter {
    framework: ChaoticSemanticFramework,
}

impl MemoryAdapter {
    pub async fn new_in_memory() -> Result<Self> {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await?;
        Ok(Self { framework })
    }

    pub async fn ingest_memory(&self, id: &str, text: &str) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("_text".to_string(), serde_json::Value::String(text.to_string()));
        self.framework.inject_text_with_metadata(id, text, metadata).await?;
        Ok(())
    }

    pub async fn query(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let hits = self.framework.probe_text(text, top_k).await?;
        Ok(hits)
    }

    pub async fn get_text(&self, id: &str) -> Result<Option<String>> {
        let concept = self.framework.get_concept(id).await?;
        Ok(concept.map(|c| {
            // Text is stored as a special metadata field
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
