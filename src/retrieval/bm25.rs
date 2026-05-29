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

// Casts are intentional for BM25 math (document counts, term frequencies)
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    total_length: usize,
    /// Inverted index mapping terms to (doc_index, term_frequency)
    postings: HashMap<Arc<str>, Vec<(usize, u32)>>,
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
                    .postings
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

        self.total_length += length;
        let idx = self.documents.len();
        self.doc_index.insert(id.to_string(), idx);

        // Update postings list for each term in the document
        for (term, &tf) in &doc.term_freqs {
            self.postings
                .entry(Arc::clone(term))
                .or_default()
                .push((idx, tf));
        }

        self.documents.push(doc);
    }

    /// Remove a document from the index.
    pub fn remove_document(&mut self, id: &str) {
        if let Some(idx) = self.doc_index.remove(id) {
            self.remove_document_at(idx);
        }
    }

    fn remove_document_at(&mut self, idx: usize) {
        // Before swapping, remove the document's terms from the postings list
        let removed_doc = &self.documents[idx];
        for term in removed_doc.term_freqs.keys() {
            if let Some(list) = self.postings.get_mut(term) {
                list.retain(|(d_idx, _)| *d_idx != idx);
            }
        }

        // Use swap_remove - gives ownership of the document
        let doc = self.documents.swap_remove(idx);
        let old_last_idx = self.documents.len();

        self.total_length = self.total_length.saturating_sub(doc.length);
        // Use owned ID to avoid clone during removal from index
        self.doc_index.remove(&doc.id);

        // If we swapped an element into idx, update its mapping and postings
        if idx < self.documents.len() {
            let swapped_doc = &self.documents[idx];
            let swapped_id = &swapped_doc.id;
            self.doc_index.insert(swapped_id.clone(), idx);

            // Update indices in postings list for the moved document
            for term in swapped_doc.term_freqs.keys() {
                if let Some(list) = self.postings.get_mut(term) {
                    for (d_idx, _) in list.iter_mut() {
                        if *d_idx == old_last_idx {
                            *d_idx = idx;
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Search for documents matching the query.
    ///
    /// Returns up to `top_k` results sorted by BM25 score (descending).
    ///
    /// Performance Optimization: Uses an inverted index (postings list) to achieve
    /// sub-linear search time. Instead of scanning all documents, we only process
    /// those containing at least one query term.
    pub fn search<T: AsRef<str>>(&self, query_tokens: &[T], top_k: usize) -> Vec<(String, f32)> {
        if self.documents.is_empty() || query_tokens.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let n = self.documents.len() as f32;
        let avgdl = self.total_length as f32 / n;

        // Pre-calculate constants for scoring (hoisted out of loop)
        let k1 = self.config.k1;
        let b = self.config.b;
        let k1_plus_1 = k1 + 1.0;
        let c1 = k1 * (1.0 - b);
        let c2 = k1 * b / avgdl;

        // Compute unique query terms and their weighted IDFs once
        let mut query_weights = Vec::with_capacity(query_tokens.len());

        // Use a set to handle duplicate tokens in query efficiently
        let mut seen_terms = HashSet::with_capacity(query_tokens.len());
        for token in query_tokens {
            let term = token.as_ref();
            if !seen_terms.insert(term) {
                continue;
            }

            // Optimization: Skip OOV terms. They contribute 0 to all scores.
            if let Some(postings) = self.postings.get(term) {
                let df = postings.len() as f32;
                if df > 0.0 {
                    let idf = ((n + 1.0) / (df + 0.5)).ln();
                    if idf > 0.0 {
                        query_weights.push((term, idf * k1_plus_1));
                    }
                }
            }
        }

        if query_weights.is_empty() {
            return Vec::new();
        }

        // Use dense accumulator for scores to maximize cache locality
        let mut doc_scores = vec![0.0f32; self.documents.len()];

        // Iterate over query terms and accumulate scores from postings lists
        for (term, weighted_idf) in query_weights {
            if let Some(postings) = self.postings.get(term) {
                for &(idx, tf) in postings {
                    let doc = &self.documents[idx];
                    let doc_len = doc.length as f32;
                    let den_base = c2.mul_add(doc_len, c1);
                    let tf = tf as f32;

                    // BM25 term score: (tf * idf * (k1 + 1)) / (tf + k1 * (1 - b + b * doc_len / avgdl))
                    let score = (tf * weighted_idf) / (tf + den_base);
                    doc_scores[idx] += score;
                }
            }
        }

        // Collect documents with non-zero scores
        let mut scores: Vec<(usize, f32)> = doc_scores
            .into_iter()
            .enumerate()
            .filter(|&(_, s)| s > 0.0)
            .collect();

        // Partial select keeps complexity near O(hits)
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

    /// Clear all documents from the index.
    pub fn clear(&mut self) {
        self.documents.clear();
        self.doc_index.clear();
        self.postings.clear();
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
#[path = "bm25/tests.rs"]
mod tests;
