use anyhow::Result;
use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
use chaotic_semantic_memory::retrieval::hybrid::{compute_weights, merge_results};
use chaotic_semantic_memory::semantic_bridge::{CanonicalConcept, ConceptGraph};
use chaotic_semantic_memory::retrieval::GraphRagConfig;
use std::collections::{HashMap, HashSet};
use tempfile::NamedTempFile;
use tokio::{fs, sync::RwLock};

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
    bridge: BridgeRetrieval,
    _tmp_db: NamedTempFile,
}

const MIN_OVERLAP_WEIGHT: f32 = 0.05;
const MIN_ADJUSTED_SCORE: f32 = 0.05;
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "am", "are", "my", "me", "i", "you", "your", "what", "which",
    "did", "do", "does", "after", "where", "number", "favorite", "current", "should",
];

impl MemoryAdapter {
    pub async fn new_in_memory() -> Result<Self> {
        let _tmp_db = tempfile::NamedTempFile::new()?;
        let db_path = _tmp_db.path().to_string_lossy().to_string();

        let framework = ChaoticSemanticFramework::builder()
            .with_local_db(db_path)
            .build()
            .await?;

        let mut graph = ConceptGraph::new();
        // Add expansion labels for bridge retrieval
        graph.add_concept(CanonicalConcept::new("bridge.color").with_label("color").with_label("hue"));
        graph.add_concept(CanonicalConcept::new("bridge.city").with_label("city").with_label("location"));

        let encoder = TextEncoder::new();
        let bridge = BridgeRetrieval::with_defaults(encoder, graph);

        Ok(Self {
            framework,
            bm25_index: RwLock::new(Bm25Index::new()),
            text_store: RwLock::new(HashMap::new()),
            bridge,
            _tmp_db,
        })
    }

    pub async fn ingest_memory(&self, id: &str, text: &str, ttl_seconds: Option<u64>) -> Result<()> {
        // Store text metadata for HDC
        let mut metadata = HashMap::new();
        metadata.insert("_text".to_string(), serde_json::Value::String(text.to_string()));

        if let Some(ttl) = ttl_seconds {
            self.framework
                .inject_text_with_ttl(id, text, ttl)
                .await?;
        } else {
            self.framework
                .inject_text_with_metadata(id, text, metadata)
                .await?;
        }

        // Tokenize and add to BM25 index
        let tokens = tokenize_for_bm25(text);
        self.bm25_index.write().await.add_document(id, &tokens);

        // Store text for retrieval
        self.text_store.write().await.insert(id.to_string(), text.to_string());

        // Automatic graph association based on proximity and overlap
        let doc_tokens = tokens;
        let mut new_associations = Vec::new();
        {
            let store = self.text_store.read().await;
            for (existing_id, existing_text) in store.iter() {
                if existing_id == id { continue; }

                let existing_tokens = tokenize_for_bm25(existing_text);
                let overlap = Self::token_overlap_ratio(&doc_tokens, &existing_tokens);

                if overlap > 0.8 {
                    new_associations.push((id.to_string(), existing_id.clone(), 0.4));
                }
            }
        }

        for (from, to, strength) in new_associations {
            let _ = self.framework.associate(&from, &to, strength).await;
        }

        Ok(())
    }

    pub async fn query(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.query_internal(text, top_k, false).await
    }

    /// Query specifically for association tasks using GraphRAG.
    pub async fn query_association(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.query_internal(text, top_k, true).await
    }

    async fn query_internal(&self, text: &str, top_k: usize, use_graph: bool) -> Result<Vec<(String, f32)>> {
        // Get HDC results
        let hdc_hits = if use_graph {
            let config = GraphRagConfig {
                anchor_top_k: top_k,
                final_top_k: top_k * 2,
                max_hops: 2,
                similarity_weight: 0.4,
                graph_weight: 0.6,
                ..Default::default()
            };
            self.framework
                .probe_text_with_graph(text, config)
                .await?
                .into_iter()
                .map(|r| (r.id, r.score))
                .collect()
        } else {
            self.framework.probe_text(text, top_k * 3).await?
        };

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
        self.query_in_session_internal(text, session_id, top_k, false).await
    }

    /// Query with session filtering specifically for association tasks.
    pub async fn query_in_session_association(&self, text: &str, session_id: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.query_in_session_internal(text, session_id, top_k, true).await
    }

    async fn query_in_session_internal(&self, text: &str, session_id: &str, top_k: usize, use_graph: bool) -> Result<Vec<(String, f32)>> {
        if use_graph {
            let all_results = self.query_association(text, top_k * 10).await?;
            let session_prefix = format!("{}:", session_id);
            let filtered: Vec<_> = all_results
                .into_iter()
                .filter(|(id, _)| id.starts_with(&session_prefix))
                .take(top_k)
                .collect();
            return Ok(filtered);
        }

        let hdc_hits = self.framework.query_in_session(text, session_id, top_k * 3).await?;
        let query_tokens = tokenize_for_bm25(text);
        let bm25_hits = self.bm25_index.read().await.search(&query_tokens, top_k * 10);
        let session_prefix = format!("{}:", session_id);
        let bm25_filtered: Vec<_> = bm25_hits.into_iter().filter(|(id, _)| id.starts_with(&session_prefix)).collect();

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

    pub async fn query_bm25(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let query_tokens = tokenize_for_bm25(text);
        let bm25_hits = self.bm25_index.read().await.search(&query_tokens, top_k);
        Ok(bm25_hits)
    }

    pub async fn query_hybrid(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        self.query(text, top_k).await
    }

    pub async fn query_bridge(&self, text: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let sing_lock = self.framework.singularity();
        let sing = sing_lock.read().await;
        let hits = self.bridge.query(&sing, text, top_k, None)?;
        Ok(hits
            .into_iter()
            .map(|h| (h.id, h.scores.final_score))
            .collect())
    }

    pub async fn query_history(
        &self,
        id: &str,
        limit: usize,
    ) -> Result<Vec<chaotic_semantic_memory::persistence::ConceptVersion>> {
        self.framework.concept_history(id, limit).await.map_err(Into::into)
    }

    pub async fn purge_expired(&self) -> Result<usize> {
        self.framework.purge_expired().await.map_err(Into::into)
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
        adapter.ingest_memory("test-1", "Hello world from memory", None).await.unwrap();

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
    async fn test_storage_bytes_reports_export_size() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();
        adapter.ingest_memory("storage-1", "hello world", None).await.unwrap();
        let bytes = adapter.storage_bytes().await.unwrap();
        assert!(bytes > 0);
    }

    #[tokio::test]
    async fn test_city_queries_return_results() {
        let adapter = MemoryAdapter::new_in_memory().await.unwrap();
        adapter
            .ingest_memory("session-123:city", "I moved to Barcelona.", None)
            .await
            .unwrap();

        let hits = adapter
            .query_in_session("What city did I move to?", "session-123", 1)
            .await
            .unwrap();
        assert_eq!(hits.first().map(|(id, _)| id.as_str()), Some("session-123:city"));
    }
}
