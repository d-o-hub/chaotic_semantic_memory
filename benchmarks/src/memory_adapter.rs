use anyhow::Result;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use chaotic_semantic_memory::retrieval::hybrid::{compute_weights, merge_results};
use std::collections::{HashMap, HashSet};
use tempfile::NamedTempFile;
use tokio::{fs, sync::RwLock};
use tracing::debug;

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
                return format!("{}e", stripped);
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
        })
    }

    pub async fn ingest_memory(&self, id: &str, text: &str) -> Result<()> {
        // Store text metadata for HDC
        let mut metadata = HashMap::new();
        metadata.insert("_text".to_string(), serde_json::Value::String(text.to_string()));
        self.framework
            .inject_text_with_metadata(id, text, metadata)
            .await?;

        // Tokenize and add to BM25 index
        let tokens = tokenize_for_bm25(text);
        self.bm25_index.write().await.add_document(id, &tokens);

        // Store text for retrieval
        self.text_store.write().await.insert(id.to_string(), text.to_string());

        if let Some(previous_id) = Self::previous_version_id(id) {
            if let Err(err) = self.framework.delete_concept(&previous_id).await {
                debug!(target: "benchmark", %previous_id, ?err, "failed to delete prior version");
            }

            self.bm25_index.write().await.remove_document(&previous_id);
            self.text_store.write().await.remove(&previous_id);
        }

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

    pub async fn storage_bytes(&self) -> Result<u64> {
        let tmp = NamedTempFile::new()?;
        let tmp_path = tmp.path().to_path_buf();
        let path_str = tmp_path.to_string_lossy().to_string();

        self.framework.export_binary(&path_str).await?;
        let bytes = fs::read(&tmp_path).await?;

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

    #[tokio::test]
    async fn test_previous_version_is_pruned() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();
        adapter
            .ingest_memory(
                "session-0001:favorite_color:v1",
                "My favorite color is blue.",
            )
            .await
            .unwrap();
        adapter
            .ingest_memory(
                "session-0001:favorite_color:v2",
                "Actually, I changed my mind. My current favorite color is green now.",
            )
            .await
            .unwrap();

        // Old concept removed
        let old = adapter
            .framework
            .get_concept("session-0001:favorite_color:v1")
            .await
            .unwrap();
        assert!(old.is_none());

        // Query should surface the v2 concept
        let hits = adapter
            .query_in_session("current favorite color", "session-0001", 1)
            .await
            .unwrap();
        assert_eq!(hits.first().map(|(id, _)| id.as_str()), Some("session-0001:favorite_color:v2"));
    }

    #[tokio::test]
    async fn test_storage_bytes_reports_export_size() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();
        adapter.ingest_memory("storage-1", "hello world").await.unwrap();
        let bytes = adapter.storage_bytes().await.unwrap();
        assert!(bytes > 0);
    }

    #[tokio::test]
    async fn test_city_queries_return_results() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();
        adapter
            .ingest_memory("session-123:city:v1", "I moved to Barcelona.")
            .await
            .unwrap();

        let hits = adapter
            .query_in_session("What city did I move to?", "session-123", 1)
            .await
            .unwrap();
        assert_eq!(hits.first().map(|(id, _)| id.as_str()), Some("session-123:city:v1"));
    }
}
