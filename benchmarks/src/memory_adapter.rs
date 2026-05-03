use anyhow::Result;
use chaotic_semantic_memory::MetadataFilter;
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use chaotic_semantic_memory::retrieval::hybrid::{compute_weights, merge_results};
use chaotic_semantic_memory::retrieval::GraphRagConfig;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// Tokenize text for BM25 matching, stripping punctuation.
fn tokenize_for_bm25(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|s| !s.is_empty() && s.len() > 1) // Skip single chars
        .map(stem_token)
        .collect()
}

fn stem_token(token: &str) -> String {
    if let Some(stripped) = token.strip_suffix("ing") {
        if stripped.len() > 2 {
            return stripped.to_string();
        }
    }

    if let Some(stripped) = token.strip_suffix("ed") {
        if stripped.len() > 2 {
            if stripped.ends_with('v') {
                return format!("{stripped}e");
            }
            return stripped.to_string();
        }
    }

    token.to_string()
}

pub struct MemoryAdapter {
    framework: ChaoticSemanticFramework,
    bm25_index: RwLock<Bm25Index>,
    text_store: RwLock<HashMap<String, String>>,
    last_session_mem: RwLock<HashMap<String, String>>,
}

const MIN_OVERLAP_WEIGHT: f32 = 0.05;
const MIN_ADJUSTED_SCORE: f32 = 0.05;
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "am", "are", "my", "me", "i", "you", "your", "what", "which",
    "did", "do", "does", "after", "where", "number", "favorite", "current", "should",
];

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
            last_session_mem: RwLock::new(HashMap::new()),
        })
    }

    pub async fn ingest_memory(&self, id: &str, text: &str) -> Result<()> {
        let session_id = id.split(':').next().unwrap_or("default");

        // Store text metadata for HDC
        let mut metadata = HashMap::new();
        metadata.insert("_text".to_string(), serde_json::Value::String(text.to_string()));
        
        // Add session_id to metadata for framework-level filtering
        metadata.insert("session_id".to_string(), serde_json::Value::String(session_id.to_string()));

        self.framework
            .inject_text_with_metadata(id, text, metadata)
            .await?;

        // Tokenize and add to BM25 index
        let tokens = tokenize_for_bm25(text);
        self.bm25_index.write().await.add_document(id, &tokens);

        // Store text for retrieval
        self.text_store.write().await.insert(id.to_string(), text.to_string());

        // Create sequential association within session for GraphRAG traversal
        {
            let mut last_mems = self.last_session_mem.write().await;
            if let Some(last_id) = last_mems.get(session_id) {
                // Bi-directional sequential link
                self.framework.associate(last_id, id, 0.8).await?;
                self.framework.associate(id, last_id, 0.4).await?;
            }
            last_mems.insert(session_id.to_string(), id.to_string());
        }

        if let Some(previous_id) = Self::previous_version_id(id) {
            // Ignore error if already deleted or not found
            let _ = self.framework.delete_concept(&previous_id).await;
            self.bm25_index.write().await.remove_document(&previous_id);
            self.text_store.write().await.remove(&previous_id);
        }

        Ok(())
    }

    pub async fn query(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        // Get HDC results using GraphRAG hybrid expansion
        let config = GraphRagConfig {
            anchor_top_k: top_k,
            max_hops: 2,
            min_assoc_strength: 0.1,
            similarity_weight: 0.7,
            graph_weight: 0.3,
            final_top_k: top_k * 3,
        };

        let hdc_results = self.framework.probe_text_with_graph(text, config).await?;
        let hdc_hits: Vec<(String, f32)> = hdc_results.into_iter().map(|r| (r.id, r.score)).collect();

        // Get BM25 results
        let query_tokens = tokenize_for_bm25(text);
        let bm25_hits = self
            .bm25_index
            .read()
            .await
            .search(&query_tokens, top_k * 10); // Search widely to allow merging

        // Compute weights based on query length
        let weights = compute_weights(query_tokens.len());

        // Filter HDC results below threshold to avoid noise
        const HDC_MIN_SCORE: f32 = 0.05;
        let hdc_filtered: Vec<_> = hdc_hits
            .into_iter()
            .filter(|(_, score)| *score >= HDC_MIN_SCORE)
            .collect();

        // Merge results
        let merged = merge_results(&bm25_hits, &hdc_filtered, weights);

        let reweighted = {
            let text_store = self.text_store.read().await;
            merged
                .into_iter()
                .filter_map(|(id, score)| {
                    let overlap = text_store.get(&id).map(|text| {
                        let doc_tokens = tokenize_for_bm25(text);
                        Self::token_overlap_ratio(&query_tokens, &doc_tokens)
                    })?;

                    let adjusted = score * (MIN_OVERLAP_WEIGHT + (1.0 - MIN_OVERLAP_WEIGHT) * overlap);
                    if adjusted < MIN_ADJUSTED_SCORE {
                        None
                    } else {
                        Some((id, adjusted))
                    }
                })
                .collect::<Vec<_>>()
        };

        Ok(reweighted.into_iter().take(top_k).collect())
    }

    /// Query with session filtering - only returns documents from the specified session.
    pub async fn query_in_session(
        &self,
        text: &str,
        session_id: &str,
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        // For standard queries, use framework-level session filtering
        let encoder = TextEncoder::new();
        let query_vec = encoder.encode(text);
        let filter = MetadataFilter::eq("session_id", session_id);
        let hdc_hits = self
            .framework
            .probe_filtered(&query_vec, top_k * 3, &filter)
            .await?;

        // Get BM25 results
        let query_tokens = tokenize_for_bm25(text);
        // Fix: Search widely and then filter by session to avoid missing relevant in-session memories
        let bm25_hits = self
            .bm25_index
            .read()
            .await
            .search(&query_tokens, 1000); // Global search

        let session_prefix = format!("{session_id}:");
        let bm25_filtered: Vec<_> = bm25_hits
            .into_iter()
            .filter(|(id, _)| id.starts_with(&session_prefix))
            .take(top_k * 3) // Apply cutoff after session filtering
            .collect();

        // Merge results
        let weights = compute_weights(query_tokens.len());
        let merged = merge_results(&bm25_filtered, &hdc_hits, weights);

        let reweighted = {
            let text_store = self.text_store.read().await;
            merged
                .into_iter()
                .filter_map(|(id, score)| {
                    let overlap = text_store.get(&id).map(|text| {
                        let doc_tokens = tokenize_for_bm25(text);
                        Self::token_overlap_ratio(&query_tokens, &doc_tokens)
                    })?;

                    let adjusted =
                        score * (MIN_OVERLAP_WEIGHT + (1.0 - MIN_OVERLAP_WEIGHT) * overlap);
                    if adjusted < MIN_ADJUSTED_SCORE {
                        None
                    } else {
                        Some((id, adjusted))
                    }
                })
                .collect::<Vec<_>>()
        };

        Ok(reweighted.into_iter().take(top_k).collect())
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

    pub async fn storage_bytes(&self) -> Result<u64> {
        // Since we are without_persistence, we can use an in-memory export if supported,
        // but for benchmarks, we'll just return 0 or implement a temp export.
        let tmp = tempfile::NamedTempFile::new()?;
        let tmp_path = tmp.path().to_path_buf();
        let path_str = tmp_path.to_string_lossy().to_string();

        self.framework.export_binary(&path_str).await?;
        let bytes = std::fs::read(&tmp_path)?;

        Ok(bytes.len() as u64)
    }

    fn previous_version_id(id: &str) -> Option<String> {
        let (prefix, version) = id.rsplit_once(":v")?;
        let version_num: u32 = version.parse().ok()?;
        if version_num == 0 || version_num == 1 {
            return None;
        }
        Some(format!("{}:v{}", prefix, version_num - 1))
    }

    fn token_overlap_ratio(query_tokens: &[String], doc_tokens: &[String]) -> f32 {
        let doc_set: HashSet<&str> = doc_tokens
            .iter()
            .map(|s| s.as_str())
            .filter(|token| !is_stopword(token))
            .collect();

        let query_terms: Vec<&str> = query_tokens
            .iter()
            .map(|s| s.as_str())
            .filter(|token| !is_stopword(token))
            .collect();

        if query_terms.is_empty() || doc_set.is_empty() {
            return 0.0;
        }

        let matches = query_terms
            .iter()
            .filter(|token| doc_set.contains(*token))
            .count();

        matches as f32 / query_terms.len() as f32
    }
}

fn is_stopword(token: &str) -> bool {
    STOPWORDS.contains(&token)
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

    #[test]
    fn test_previous_version_id() {
        assert_eq!(
            MemoryAdapter::previous_version_id("session-0001:favorite_color:v2"),
            Some("session-0001:favorite_color:v1".to_string())
        );
        assert_eq!(
            MemoryAdapter::previous_version_id("session-0001:favorite_color:v1"),
            None
        );
        assert_eq!(
            MemoryAdapter::previous_version_id("session-0001:favorite_color"),
            None
        );
    }
}
