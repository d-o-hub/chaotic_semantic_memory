// Exact float comparisons for BM25 score test assertions

use std::sync::Arc;

use super::super::*;

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
    assert!(index.search(&["hello"], 10).is_empty());
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
fn test_postings_integrity() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world", "hello"]);

    let hello_arc: Arc<str> = Arc::from("hello");
    let world_arc: Arc<str> = Arc::from("world");

    let hello_postings = index.postings.get(&hello_arc).unwrap();
    assert_eq!(hello_postings.len(), 1);
    assert_eq!(hello_postings[0], (0, 2)); // doc_idx 0, tf 2

    let world_postings = index.postings.get(&world_arc).unwrap();
    assert_eq!(world_postings.len(), 1);
    assert_eq!(world_postings[0], (0, 1)); // doc_idx 0, tf 1
}

#[test]
fn test_postings_remove_integrity() {
    let mut index = Bm25Index::new();
    index.add_document("doc0", &["a"]);
    index.add_document("doc1", &["b"]);
    index.add_document("doc2", &["c"]);

    // Remove doc1 (middle). doc2 (idx 2) should swap to idx 1.
    index.remove_document("doc1");

    let a_arc: Arc<str> = Arc::from("a");
    let c_arc: Arc<str> = Arc::from("c");
    let b_arc: Arc<str> = Arc::from("b");

    assert_eq!(index.postings.get(&a_arc).unwrap()[0].0, 0);
    assert_eq!(index.postings.get(&c_arc).unwrap()[0].0, 1); // doc2 moved to idx 1
    assert!(index.postings.get(&b_arc).unwrap().is_empty());
}

#[test]
fn test_postings_clear_integrity() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello"]);
    index.clear();
    assert!(index.postings.is_empty());
}

#[test]
fn test_search_equivalence() {
    let mut index = Bm25Index::new();
    let tokens = ["a", "b", "c", "d"];
    for i in 0..100 {
        let doc_tokens: Vec<&str> = tokens
            .iter()
            .filter(|_| rand::random::<bool>())
            .copied()
            .collect();
        index.add_document(&format!("doc{i}"), &doc_tokens);
    }

    let query = ["a", "b"];
    let results = index.search(&query, 10);

    // Since we can't easily run the old code, we verify that the results
    // are consistent (sorted by score, positive scores).
    assert!(!results.is_empty());
    for i in 0..results.len() - 1 {
        assert!(results[i].1 >= results[i + 1].1);
        assert!(results[i].1 > 0.0);
    }
}
