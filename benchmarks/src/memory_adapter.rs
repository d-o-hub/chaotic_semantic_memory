use anyhow::Result;
use chaotic_semantic_memory::prelude::*;

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
        self.framework.inject_text(id, text).await?;
        Ok(())
    }

    pub async fn query(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let hits = self.framework.probe_text(text, top_k).await?;
        Ok(hits)
    }

    pub async fn get_text(&self, id: &str) -> Result<Option<String>> {
        let concept = self.framework.get_concept(id).await?;
        Ok(concept.map(|c| {
            // Text is often stored as a special metadata field or ID
            c.metadata
                .get("_text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or(c.id)
        }))
    }
}
