// Exact float comparisons for BM25 score test assertions

use super::super::*;
use std::sync::Arc;

#[test]
fn test_add_document() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world"]);
    assert_eq!(index.len(), 1);

    // Internal state verification
    assert_eq!(index.doc_lengths.len(), 1);
    assert_eq!(index.doc_lengths[0], 2);
    let hello = Arc::from("hello");
    assert!(index.postings.contains_key(&hello));
    assert_eq!(index.postings.get(&hello).unwrap()[0], (0, 1));
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

    // Verify postings integrity
    let hello = Arc::from("hello");
    let list = index.postings.get(&hello).unwrap();
    assert_eq!(list.len(), 1);
    // doc1 was index 0, doc2 was index 1.
    // swap_remove(0) moved doc2 to index 0.
    assert_eq!(list[0].0, 0);
}

#[test]
fn test_replace_document() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world"]);
    index.add_document("doc1", &["goodbye", "rust"]);

    assert_eq!(index.len(), 1);

    let results = index.search(&["rust"], 10);
    assert_eq!(results[0].0, "doc1");

    let results_old = index.search(&["hello"], 10);
    assert!(results_old.is_empty());
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
    // Long document with same term but more other words
    index.add_document(
        "long",
        &[
            "hello", "other", "words", "here", "more", "words", "to", "make", "it", "long",
        ],
    );

    // Both match, but shorter doc should score higher per-term
    // (BM25 normalizes by document length)
    let results = index.search(&["hello"], 10);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "short");
}

#[test]
fn test_clear() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world"]);
    index.clear();
    assert!(index.is_empty());
    assert!(index.search(&["hello"], 10).is_empty());
    assert!(index.postings.is_empty());
    assert!(index.doc_lengths.is_empty());
}

#[test]
fn test_avg_doc_length() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["a", "b", "c"]);
    index.add_document("doc2", &["x", "y"]);

    assert!((index.avg_doc_length() - 2.5).abs() < 1e-6);
}

#[test]
fn test_custom_config() {
    let config = Bm25Config { k1: 2.0, b: 0.5 };
    let index = Bm25Index::with_config(config);
    assert!((index.config.k1 - 2.0).abs() < 1e-6);
    assert!((index.config.b - 0.5).abs() < 1e-6);
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
    assert!(index.search(&["rust"], 10).is_empty());
}

#[test]
fn test_swap_remove_integrity_complex() {
    let mut index = Bm25Index::new();
    index.add_document("doc0", &["a", "b"]);
    index.add_document("doc1", &["b", "c"]);
    index.add_document("doc2", &["c", "d"]);
    index.add_document("doc3", &["a", "d"]);

    // Remove doc1 (index 1). doc3 (index 3) should move to index 1.
    index.remove_document("doc1");

    assert_eq!(index.len(), 3);
    assert_eq!(index.doc_index.get("doc3"), Some(&1));
    assert_eq!(index.doc_lengths[1], 2);

    // Verify doc3 postings point to new index 1
    let a = Arc::from("a");
    let d = Arc::from("d");
    assert!(
        index
            .postings
            .get(&a)
            .unwrap()
            .iter()
            .any(|&(idx, _)| idx == 1)
    );
    assert!(
        index
            .postings
            .get(&d)
            .unwrap()
            .iter()
            .any(|&(idx, _)| idx == 1)
    );

    // Search should still work for relocated doc3
    let results = index.search(&["d"], 10);
    assert_eq!(results[0].0, "doc3");
}

#[test]
fn test_scoring_formula_exactness() {
    let mut index = Bm25Index::with_config(Bm25Config { k1: 1.2, b: 0.75 });
    index.add_document("doc1", &["hello", "world"]);

    // n = 1, avgdl = 2.0
    // query "hello": df("hello") = 1
    // idf = ln((1+1)/(1+0.5)) = ln(2/1.5) = ln(1.333...) = 0.287682
    // weighted_idf = idf * (1.2 + 1) = 0.287682 * 2.2 = 0.6329
    // doc1: tf=1, len=2
    // den_base = 1.2 * (1 - 0.75 + 0.75 * 2 / 2.0) = 1.2 * (0.25 + 0.75) = 1.2
    // score = (1 * 0.6329) / (1 + 1.2) = 0.6329 / 2.2 = 0.287682

    let results = index.search(&["hello"], 10);
    assert!((results[0].1 - 0.287682).abs() < 1e-6);
}

#[test]
fn test_multi_term_scoring() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["apple", "banana"]);
    index.add_document("doc2", &["apple", "cherry"]);

    // Both docs contain "apple". Only doc1 contains "banana".
    // Score for doc1 should be higher for query ["apple", "banana"] than doc2.
    let results = index.search(&["apple", "banana"], 10);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "doc1");
    assert!(results[0].1 > results[1].1);
}
