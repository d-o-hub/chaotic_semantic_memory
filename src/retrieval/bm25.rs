//! BM25 keyword search index for hybrid retrieval.
//! Implements Okapi BM25 for exact keyword matching.

// Casts are intentional for BM25 math (document counts, term frequencies)
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use csm_traits::{AbsenceEntry, AbsenceStore};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

/// Maximum top_k limit for retrieval operations to prevent CWE-770 memory exhaustion.
pub const MAX_TOP_K_LIMIT: usize = 100_000;

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

thread_local! {
    static DOC_SCORES_BUFFER: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static SCORES_COLLECT_BUFFER: RefCell<Vec<(usize, f32)>> = const { RefCell::new(Vec::new()) };
    static TOUCHED_INDICES_BUFFER: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Default, Clone)]
struct Bm25Cache {
    /// Cached normalization factors (doc_term_b, tf1_recip) for fast scoring.
    /// Using a single vector of pairs improves cache locality in the scoring hot loop.
    factors: Vec<(f32, f32)>,
}

#[derive(Debug)]
/// BM25-based document index for keyword search.
pub struct Bm25Index {
    config: Bm25Config,
    documents: Vec<Document>,
    doc_index: HashMap<String, usize>,
    /// Inverted index mapping terms to (document_index, term_frequency)
    postings: HashMap<Arc<str>, Vec<(usize, u32)>>,
    /// Cached normalization factors for fast scoring.
    norm_cache: RwLock<Bm25Cache>,
    /// True when scoring factors are stale due to index mutations.
    norm_cache_dirty: AtomicBool,
    /// Document lengths for fast access in scoring loop
    doc_lengths: Vec<f32>,
    total_length: usize,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self {
            config: Bm25Config::default(),
            documents: Vec::new(),
            doc_index: HashMap::new(),
            postings: HashMap::new(),
            norm_cache: RwLock::new(Bm25Cache::default()),
            norm_cache_dirty: AtomicBool::new(true),
            doc_lengths: Vec::new(),
            total_length: 0,
        }
    }
}

/// Returns true if the query has a known absence record with
/// attempt_count >= min_attempts, indicating BM25 should be skipped.
/// Called by the framework probe paths to skip retrieval for queries that
/// repeatedly abstained.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub async fn is_known_absent(query: &str, store: &dyn AbsenceStore, min_attempts: u32) -> bool {
    let id = AbsenceEntry::id_for(query);
    match store.get_absence(&id).await {
        Ok(Some(entry)) => entry.attempt_count >= min_attempts,
        _ => false,
    }
}

#[allow(clippy::expect_used)] // Lock poisoning is unrecoverable
impl Clone for Bm25Index {
    fn clone(&self) -> Self {
        Self {
            config: self.config,
            documents: self.documents.clone(),
            doc_index: self.doc_index.clone(),
            postings: self.postings.clone(),
            norm_cache: RwLock::new(
                self.norm_cache
                    .read()
                    .expect("Bm25Index norm_cache lock poisoned")
                    .clone(),
            ),
            norm_cache_dirty: AtomicBool::new(self.norm_cache_dirty.load(AtomicOrdering::Acquire)),
            doc_lengths: self.doc_lengths.clone(),
            total_length: self.total_length,
        }
    }
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
        if let Some(idx) = self.doc_index.remove(id) {
            self.remove_document_at(idx);
        }

        let mut term_freqs = HashMap::with_capacity(tokens.len().min(100));
        for token in tokens {
            let term = token.as_ref();
            // Arc interning - share term strings between documents and postings
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

        // Update postings list
        for (term, &tf) in &doc.term_freqs {
            self.postings
                .entry(Arc::clone(term))
                .or_default()
                .push((idx, tf));
        }

        self.doc_lengths.push(length as f32);
        self.documents.push(doc);
        self.norm_cache_dirty.store(true, AtomicOrdering::Release);
    }

    /// Remove a document from the index.
    pub fn remove_document(&mut self, id: &str) {
        if let Some(idx) = self.doc_index.remove(id) {
            self.remove_document_at(idx);
        }
    }

    fn remove_document_at(&mut self, idx: usize) {
        let last_idx = self.documents.len() - 1;

        // Use swap_remove - gives ownership of the document
        let doc = self.documents.swap_remove(idx);
        self.doc_lengths.swap_remove(idx);

        // Update postings and cleanup empty entries
        for term in doc.term_freqs.keys() {
            let mut needs_removal = false;
            if let Some(entries) = self.postings.get_mut(term) {
                if let Some(p_idx) = entries.iter().position(|&(d_idx, _)| d_idx == idx) {
                    entries.swap_remove(p_idx);
                }
                needs_removal = entries.is_empty();
            }
            if needs_removal {
                self.postings.remove(term);
            }
        }

        self.total_length = self.total_length.saturating_sub(doc.length);

        // If we swapped an element into idx, update its mapping
        if idx < self.documents.len() {
            let swapped_id = &self.documents[idx].id;
            self.doc_index.insert(swapped_id.clone(), idx);

            // Update indices in postings list for the swapped document
            for term in self.documents[idx].term_freqs.keys() {
                if let Some(entries) = self.postings.get_mut(term) {
                    if let Some(p_idx) = entries.iter().position(|&(d_idx, _)| d_idx == last_idx) {
                        entries[p_idx].0 = idx;
                    }
                }
            }
        }
        self.norm_cache_dirty.store(true, AtomicOrdering::Release);
    }

    /// Search for documents matching the query.
    ///
    /// Returns up to `top_k` results sorted by BM25 score (descending).
    // SAFETY: The read guard must span the entire scoring loop to guarantee
    // cache vector stability. Tightening the drop would require re-acquiring
    // the lock mid-loop, which is both slower and semantically incorrect.
    #[allow(clippy::significant_drop_tightening)]
    #[allow(clippy::expect_used)]
    pub fn search<T: AsRef<str>>(&self, query_tokens: &[T], top_k: usize) -> Vec<(String, f32)> {
        let top_k = top_k.min(MAX_TOP_K_LIMIT);
        if top_k == 0 || query_tokens.is_empty() || self.is_empty() {
            return Vec::new();
        }

        let n = self.len() as f32;
        let num_docs = self.documents.len();
        let n_plus_1 = n + 1.0;
        // Optimization: Hoist ln(N+1) to replace division with subtraction in IDF calculation.
        let n_plus_1_ln = n_plus_1.ln();

        // Pre-calculate constants for scoring (hoisted out of loop)
        let k1 = self.config.k1;
        let k1_plus_1 = k1 + 1.0;

        // Compute unique query terms and their weighted IDFs once.
        // Optimization: Pre-fetch postings list references to minimize HashMap lookups in the scoring loop.
        let mut query_weights = Vec::with_capacity(query_tokens.len());

        // Fast-path for query deduplication: linear scan for short queries (<= 8 tokens).
        if query_tokens.len() <= 8 {
            let mut terms = [None; 8];
            let mut count = 0;
            for token in query_tokens {
                let term = token.as_ref();
                let mut duplicate = false;
                // Clippy note: linear scan on stack-allocated array is faster than hashing for small queries.
                #[allow(clippy::needless_range_loop)]
                for i in 0..count {
                    if terms[i] == Some(term) {
                        duplicate = true;
                        break;
                    }
                }
                if !duplicate {
                    terms[count] = Some(term);
                    count += 1;
                    self.push_query_weight(term, n_plus_1_ln, k1_plus_1, &mut query_weights);
                }
            }
        } else {
            let mut seen_terms = HashSet::with_capacity(query_tokens.len());
            for token in query_tokens {
                let term = token.as_ref();
                if seen_terms.insert(term) {
                    self.push_query_weight(term, n_plus_1_ln, k1_plus_1, &mut query_weights);
                }
            }
        }

        if query_weights.is_empty() {
            return Vec::new();
        }

        // Optimization: Use thread-local buffers to eliminate O(N) allocation and zeroing per search call.
        TOUCHED_INDICES_BUFFER.with(|touched_buffer| {
            let mut touched_indices = touched_buffer.borrow_mut();
            let touched_indices = &mut *touched_indices;
            touched_indices.clear();

            DOC_SCORES_BUFFER.with(|buffer| {
                let mut doc_scores = buffer.borrow_mut();
                let doc_scores = &mut *doc_scores;
                // Always clear the active prefix (resize alone does not zero old slots).
                if doc_scores.len() < num_docs {
                    doc_scores.resize(num_docs, 0.0);
                }
                doc_scores[..num_docs].fill(0.0);

                // Optimization: Acquire read lock once. Only invoke write-locking `ensure_norm_cache` if cache is dirty, avoiding double lock acquisition.
                let mut cache_guard = self
                    .norm_cache
                    .read()
                    .expect("Bm25Index norm_cache lock poisoned");

                if self.norm_cache_dirty.load(AtomicOrdering::Acquire) {
                    drop(cache_guard);
                    self.ensure_norm_cache();
                    cache_guard = self
                        .norm_cache
                        .read()
                        .expect("Bm25Index norm_cache lock poisoned");
                }
                {
                    let factors = cache_guard.factors.as_slice();
                    debug_assert_eq!(factors.len(), num_docs);

                    for (weighted_idf, entries) in query_weights {
                        if weighted_idf <= 0.0 {
                            continue;
                        }

                        for &(doc_idx, tf) in entries {
                            // SAFETY: doc_idx from postings; buffers sized to num_docs.
                            unsafe {
                                let score_ptr = doc_scores.get_unchecked_mut(doc_idx);
                                if *score_ptr == 0.0 {
                                    touched_indices.push(doc_idx);
                                }
                                let (doc_term_b, tf1_recip) = *factors.get_unchecked(doc_idx);
                                if tf == 1 {
                                    *score_ptr += weighted_idf * tf1_recip;
                                } else {
                                    let tf = tf as f32;
                                    *score_ptr += (tf * weighted_idf) / (tf + doc_term_b);
                                }
                            }
                        }
                    }
                }

                SCORES_COLLECT_BUFFER.with(|collect_buffer| {
                    let mut scores = collect_buffer.borrow_mut();
                    let scores = &mut *scores;
                    scores.clear();

                    for &idx in touched_indices.iter() {
                        // SAFETY: idx is derived from touched_indices which only contains
                        // valid indices into doc_scores pushed during the scoring loop.
                        debug_assert!(idx < doc_scores.len());
                        let score = unsafe { *doc_scores.get_unchecked(idx) };
                        // Reset buffer slot and collect. The score > 0.0 guard handles
                        // the theoretical duplicate-index case (all real accumulations
                        // are strictly positive, so duplicates are near-impossible).
                        if score > 0.0 {
                            scores.push((idx, score));
                            debug_assert!(idx < doc_scores.len());
                            unsafe { *doc_scores.get_unchecked_mut(idx) = 0.0 };
                        }
                    }

                    // Partial select keeps complexity near O(T) (touched documents) for large corpora
                    if scores.len() > top_k {
                        let nth = top_k - 1;
                        scores.select_nth_unstable_by(nth, score_cmp_desc);
                        scores.truncate(top_k);
                    }
                    scores.sort_unstable_by(score_cmp_desc);

                    // Map to final results, cloning IDs only for top_k.
                    // Safe indexing: idx comes from scores which was built from touched_indices,
                    // all of which are valid document indices by the postings-to-documents invariant.
                    scores
                        .iter()
                        .map(|&(idx, score)| (self.documents[idx].id.clone(), score))
                        .collect()
                })
            })
        })
    }

    #[inline]
    fn push_query_weight<'a>(
        &'a self,
        term: &'a str,
        n_plus_1_ln: f32,
        k1_plus_1: f32,
        query_weights: &mut Vec<(f32, &'a [(usize, u32)])>,
    ) {
        if let Some(postings) = self.postings.get(term) {
            let df = postings.len() as f32;
            // Optimization: Use ln(N + 1.0) - ln(df + 0.5) to avoid division.
            // idf = ln((N + 1.0) / (df + 0.5)) is always > 0 for all N >= df >= 1
            let idf = n_plus_1_ln - (df + 0.5).ln();
            query_weights.push((idf * k1_plus_1, postings));
        }
    }

    /// Clear all documents from the index.
    pub fn clear(&mut self) {
        self.documents.clear();
        self.doc_lengths.clear();
        self.doc_index.clear();
        self.postings.clear();
        self.total_length = 0;
        self.norm_cache_dirty.store(true, AtomicOrdering::Release);
    }

    /// Get the number of documents in the index.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    #[allow(clippy::significant_drop_tightening)]
    #[allow(clippy::expect_used)]
    fn ensure_norm_cache(&self) {
        let n_docs = self.documents.len();
        if !self.norm_cache_dirty.load(AtomicOrdering::Acquire) {
            let cache = self
                .norm_cache
                .read()
                .expect("Bm25Index norm_cache lock poisoned");
            // Rebuild if length drifted even when dirty flag is clear.
            if cache.factors.len() == n_docs {
                return;
            }
        }

        if self.is_empty() {
            self.norm_cache_dirty.store(false, AtomicOrdering::Release);
            return;
        }

        let mut cache = self
            .norm_cache
            .write()
            .expect("Bm25Index norm_cache lock poisoned");

        if !self.norm_cache_dirty.load(AtomicOrdering::Acquire) && cache.factors.len() == n_docs {
            return;
        }

        let n = self.len() as f32;
        let avgdl = self.total_length as f32 / n;
        let k1 = self.config.k1;
        let b = self.config.b;
        let c1 = k1 * (1.0 - b);
        let c2 = k1 * b / avgdl;

        cache.factors.clear();
        cache.factors.reserve(self.doc_lengths.len());

        for &doc_len in &self.doc_lengths {
            let b_val = c2.mul_add(doc_len, c1);
            // Optimization: Use recip() to potentially leverage hardware reciprocal instructions.
            cache.factors.push((b_val, (1.0 + b_val).recip()));
        }

        self.norm_cache_dirty.store(false, AtomicOrdering::Release);
    }

    /// Get the average document length.
    pub fn avg_doc_length(&self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            self.total_length as f32 / self.documents.len() as f32
        }
    }
}

fn score_cmp_desc(a: &(usize, f32), b: &(usize, f32)) -> Ordering {
    // Score desc, then doc index asc — stable under equal scores / sort_unstable.
    b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0))
}

#[cfg(test)]
#[path = "bm25/tests.rs"]
mod tests;

#[cfg(all(test, not(target_arch = "wasm32"), feature = "persistence"))]
#[path = "bm25/absence_short_circuit_tests.rs"]
mod absence_short_circuit_tests;
