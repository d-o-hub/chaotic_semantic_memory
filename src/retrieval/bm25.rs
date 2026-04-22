//! BM25 keyword search index for hybrid retrieval.
//!
//! Implements the Okapi BM25 ranking function for exact keyword matching.
//! Used alongside HDC semantic search for improved short-query recall.
//!
//! # Algorithm
//!
//! BM25 scores documents based on:
//! - Term frequency (TF) with saturation parameter k1
//! - Inverse document frequency (IDF)
//! - Document length normalization with parameter b
//!
//! # Example
//!
//! ```
//! use chaotic_semantic_memory::retrieval::bm25::Bm25Index;
//!
//! let mut index = Bm25Index::new();
//! index.add_document("doc1", &["hello", "world"]);
//! index.add_document("doc2", &["hello", "rust"]);
//!
//! let results = index.search(&["hello", "world"], 10);
//! assert_eq!(results[0].0, "doc1"); // Exact match ranks first
//! ```

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

/// Configuration for BM25 ranking algorithm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bm25Config {
    /// Controls term frequency saturation. Typical value: 1.2.
    pub k1: f32,
    /// Controls document length normalization. Typical value: 0.75.
    pub b: f32,
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self { k1: 1.2, b: 0.75 }
    }
}

/// A document in the BM25 index.
#[derive(Debug, Clone)]
struct Document {
    id: String,
    term_freqs: HashMap<Arc<str>, u32>,
    length: usize,
}

/// BM25-based document index for keyword search.
#[derive(Debug, Clone, Default)]
pub struct Bm25Index {
    config: Bm25Config,
    documents: Vec<Document>,
    doc_index: HashMap<String, usize>,
    doc_freqs: HashMap<Arc<str>, u32>,
    total_length: usize,
}

impl Bm25Index {
    /// Create a new BM25 index with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new BM25 index with custom configuration.
    pub fn with_config(config: Bm25Config) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Add a document to the index.
    ///
    /// If a document with the same ID already exists, it will be replaced.
    pub fn add_document<T: AsRef<str>>(&mut self, id: &str, tokens: &[T]) {
        if self.doc_index.contains_key(id) {
            self.remove_document(id);
        }

        let mut term_freqs = HashMap::with_capacity(tokens.len().min(100));
        for token in tokens {
            let term = token.as_ref();
            // Arc interning - share term strings between documents and doc_freqs
            // Double lookup pattern to bypass lack of get_key_value_mut
            if let Some(count) = term_freqs.get_mut(term) {
                *count += 1;
            } else {
                // If term exists in index, reuse its Arc to save memory
                let term_arc = self
                    .doc_freqs
                    .get_key_value(term)
                    .map_or_else(|| Arc::from(term), |(k, _)| Arc::clone(k));

                term_freqs.insert(term_arc, 1);
            }
        }

        let length = tokens.len();
        let doc = Document {
            id: id.to_string(),
            term_freqs,
            length,
        };

        // Update global document frequencies
        for term in doc.term_freqs.keys() {
            *self.doc_freqs.entry(Arc::clone(term)).or_insert(0) += 1;
        }

        self.total_length += length;
        let idx = self.documents.len();
        self.doc_index.insert(id.to_string(), idx);
        self.documents.push(doc);
    }

    /// Remove a document from the index.
    pub fn remove_document(&mut self, id: &str) {
        if let Some(idx) = self.doc_index.remove(id) {
            self.remove_document_at(idx);
        }
    }

    fn remove_document_at(&mut self, idx: usize) {
        // Use swap_remove - gives ownership of the document
        let doc = self.documents.swap_remove(idx);

        // Update document frequencies
        for term in doc.term_freqs.keys() {
            if let Some(df) = self.doc_freqs.get_mut(term) {
                *df = df.saturating_sub(1);
            }
        }

        self.total_length = self.total_length.saturating_sub(doc.length);
        // Use owned ID to avoid clone during removal from index
        self.doc_index.remove(&doc.id);

        // If we swapped an element into idx, update its mapping
        if idx < self.documents.len() {
            let swapped_id = &self.documents[idx].id;
            self.doc_index.insert(swapped_id.clone(), idx);
        }
    }

    /// Search for documents matching the query.
    ///
    /// Returns up to `top_k` results sorted by BM25 score (descending).
    pub fn search<T: AsRef<str>>(&self, query_tokens: &[T], top_k: usize) -> Vec<(String, f32)> {
        if self.documents.is_empty() || query_tokens.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let n = self.documents.len() as f32;
        let avgdl = self.total_length as f32 / n;

        // Compute unique query terms and their IDFs once
        let mut query_terms = Vec::with_capacity(query_tokens.len());
        let mut idf_values = Vec::with_capacity(query_tokens.len());

        // Use a set to handle duplicate tokens in query efficiently
        let mut seen_terms = HashSet::with_capacity(query_tokens.len());
        for token in query_tokens {
            let term = token.as_ref();
            if !seen_terms.insert(term) {
                continue;
            }
            let df = self.doc_freqs.get(term).copied().unwrap_or(0) as f32;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            if idf > 0.0 {
                query_terms.push(term);
                idf_values.push(idf);
            }
        }

        if query_terms.is_empty() {
            return Vec::new();
        }

        // Pre-calculate constants for scoring (hoisted out of loop)
        let k1 = self.config.k1;
        let b = self.config.b;
        let k1_plus_1 = k1 + 1.0;
        let c1 = k1 * (1.0 - b);
        let c2 = k1 * b / avgdl;

        // Pre-calculate weighted IDF values: idf * (k1 + 1)
        let weighted_idf: Vec<f32> = idf_values.iter().map(|&idf| idf * k1_plus_1).collect();

        // Score each document - store index to avoid String clones (parallel when available)
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(usize, f32)> = self
            .documents
            .par_iter()
            .enumerate()
            .map(|(idx, doc)| {
                let score = self.score_document(doc, &query_terms, &weighted_idf, c1, c2);
                (idx, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(usize, f32)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                let score = self.score_document(doc, &query_terms, &weighted_idf, c1, c2);
                (idx, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Partial select keeps complexity near O(n) for large corpora
        if scores.len() > top_k {
            let nth = top_k - 1;
            scores.select_nth_unstable_by(nth, score_cmp_desc);
            scores.truncate(top_k);
        }
        scores.sort_unstable_by(score_cmp_desc);

        // Map to final results, cloning IDs only for top_k
        scores
            .into_iter()
            .map(|(idx, score)| (self.documents[idx].id.clone(), score))
            .collect()
    }

    fn score_document(
        &self,
        doc: &Document,
        query_terms: &[&str],
        weighted_idf: &[f32],
        c1: f32,
        c2: f32,
    ) -> f32 {
        let mut score = 0.0;
        let doc_len = doc.length as f32;

        // Document-length normalization denominator base (constant for all terms in this doc)
        let den_base = c2.mul_add(doc_len, c1);

        for (i, term) in query_terms.iter().enumerate() {
            // Skip terms not in document
            let tf = match doc.term_freqs.get(*term) {
                Some(&tf) => tf as f32,
                None => continue,
            };

            // BM25 term score using pre-calculated constants:
            // score = idf * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * doc_len / avgdl))
            // denominator = tf + k1 * (1 - b) + (k1 * b / avgdl) * doc_len
            let numerator = tf * weighted_idf[i];
            let denominator = tf + den_base;

            score += numerator / denominator;
        }

        score
    }

    /// Clear all documents from the index.
    pub fn clear(&mut self) {
        self.documents.clear();
        self.doc_index.clear();
        self.doc_freqs.clear();
        self.total_length = 0;
    }

    /// Get the number of documents in the index.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Get the average document length.
    pub fn avg_doc_length(&self) -> f32 {
        if self.documents.is_empty() {
            0.0
        } else {
            self.total_length as f32 / self.documents.len() as f32
        }
    }
}

fn score_cmp_desc(a: &(usize, f32), b: &(usize, f32)) -> Ordering {
    b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_document() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_search_exact_match() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.add_document("doc2", &["hello", "rust"]);

        let results = index.search(&["hello", "world"], 10);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_search_partial_match() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.add_document("doc2", &["goodbye", "world"]);

        let results = index.search(&["hello"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_search_empty_index() {
        let index = Bm25Index::new();
        let results = index.search(&["hello"], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);

        let results: Vec<(String, f32)> = index.search::<&str>(&[], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_remove_document() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.add_document("doc2", &["hello", "rust"]);

        index.remove_document("doc1");
        assert_eq!(index.len(), 1);

        let results = index.search(&["world"], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_replace_document() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.add_document("doc1", &["goodbye", "rust"]);

        assert_eq!(index.len(), 1);

        let results = index.search(&["rust"], 10);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_top_k() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.add_document("doc2", &["hello", "rust"]);
        index.add_document("doc3", &["hello", "python"]);

        let results = index.search(&["hello"], 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_top_k_zero_returns_empty() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);

        let results = index.search(&["hello"], 0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_idf_rare_term_higher_score() {
        let mut index = Bm25Index::new();

        // "rare" appears in 1 doc, "common" appears in 3 docs
        index.add_document("doc1", &["rare", "common"]);
        index.add_document("doc2", &["common"]);
        index.add_document("doc3", &["common"]);

        // Searching for both should rank doc1 higher (contains rare term)
        let results = index.search(&["rare", "common"], 10);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_doc_length_normalization() {
        let mut index = Bm25Index::new();

        // Short document with term
        index.add_document("short", &["hello"]);
        // Long document with same term repeated
        index.add_document(
            "long",
            &[
                "hello", "hello", "hello", "hello", "hello", "other", "words", "here",
            ],
        );

        // Both match, but shorter doc should score higher per-term
        // (BM25 normalizes by document length)
        let results = index.search(&["hello"], 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.clear();

        assert!(index.is_empty());
        let results = index.search(&["hello"], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_avg_doc_length() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["a", "b", "c"]);
        index.add_document("doc2", &["x", "y"]);

        assert_eq!(index.avg_doc_length(), 2.5);
    }

    #[test]
    fn test_custom_config() {
        let config = Bm25Config { k1: 2.0, b: 0.5 };
        let index = Bm25Index::with_config(config);
        assert_eq!(index.config.k1, 2.0);
        assert_eq!(index.config.b, 0.5);
    }

    #[test]
    fn test_zero_length_document() {
        let mut index = Bm25Index::new();
        index.add_document("empty", &[] as &[&str]);
        index.add_document("doc1", &["hello"]);

        let results = index.search(&["hello"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_single_term_query() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        let results = index.search(&["hello"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_no_matching_terms() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        let results = index.search(&["rust"], 10);
        assert!(results.is_empty());
    }
}
