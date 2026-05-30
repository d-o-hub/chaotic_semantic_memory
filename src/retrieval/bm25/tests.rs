// Exact float comparisons for BM25 score test assertions

use super::super::*;
use std::collections::HashMap;
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
fn test_postings_integrity_strict() {
    let mut index = Bm25Index::new();
    // doc0: "a" (tf=1)
    // doc1: "a" (tf=2), "b" (tf=1)
    // doc2: "a" (tf=3), "b" (tf=2), "c" (tf=1)
    index.add_document("doc0", &["a"]);
    index.add_document("doc1", &["a", "a", "b"]);
    index.add_document("doc2", &["a", "a", "a", "b", "b", "c"]);

    let a = Arc::from("a");
    let b = Arc::from("b");
    let c = Arc::from("c");

    // Initial state
    assert_eq!(index.postings.get(&a).unwrap().len(), 3);
    assert_eq!(index.postings.get(&b).unwrap().len(), 2);
    assert_eq!(index.postings.get(&c).unwrap().len(), 1);

    // Remove doc1 (idx 1). doc2 (idx 2) moves to idx 1.
    index.remove_document("doc1");

    // doc0: idx 0, "a" (tf=1)
    // doc2: idx 1, "a" (tf=3), "b" (tf=2), "c" (tf=1)

    let a_list = index.postings.get(&a).unwrap();
    assert_eq!(a_list.len(), 2);
    let mut a_map: HashMap<u32, u32> = HashMap::new();
    for &(idx, tf) in a_list {
        a_map.insert(idx, tf);
    }
    assert_eq!(a_map.get(&0), Some(&1)); // doc0
    assert_eq!(a_map.get(&1), Some(&3)); // doc2 relocated
    assert!(!a_map.contains_key(&2));

    let b_list = index.postings.get(&b).unwrap();
    assert_eq!(b_list.len(), 1);
    assert_eq!(b_list[0], (1, 2)); // doc2 relocated

    let c_list = index.postings.get(&c).unwrap();
    assert_eq!(c_list.len(), 1);
    assert_eq!(c_list[0], (1, 1)); // doc2 relocated
}

#[test]
fn test_search_score_exactness_hardened() {
    let mut index = Bm25Index::with_config(Bm25Config { k1: 2.0, b: 0.5 });
    // doc0: 10 "a"s, len 10
    let tokens = vec!["a"; 10];
    index.add_document("doc0", &tokens);

    // n = 1, avgdl = 10.0
    // query "a": df("a") = 1
    // idf = ln((1+1)/(1+0.5)) = 0.287682
    // weighted_idf = idf * (k1 + 1) = 0.287682 * 3.0 = 0.863046
    // den_base = k1 * (1 - b + b * doc_len / avgdl) = 2.0 * (0.5 + 0.5 * 10 / 10.0) = 2.0 * 1.0 = 2.0
    // score = (tf * weighted_idf) / (tf + den_base) = (10 * 0.863046) / (10 + 2.0) = 8.63046 / 12.0 = 0.719205

    let results = index.search(&["a"], 10);
    assert!((results[0].1 - 0.719205).abs() < 1e-6);
}

#[test]
fn test_complex_multi_doc_precise_scoring() {
    let config = Bm25Config { k1: 1.2, b: 0.75 };
    let mut index = Bm25Index::with_config(config);

    index.add_document("doc0", &["a", "b"]);
    index.add_document("doc1", &["a", "c"]);

    // query ["a", "b"]
    // idf(a) = 0.1823215, idf(b) = 0.6931472
    // weighted_idf(a) = 0.4011073, weighted_idf(b) = 1.5249238
    // doc0: total = 0.1823215 + 0.6931472 = 0.8754687
    // doc1: total = 0.1823215

    let results = index.search(&["a", "b"], 10);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "doc0");
    assert!((results[0].1 - 0.8754687).abs() < 1e-6);
    assert_eq!(results[1].0, "doc1");
    assert!((results[1].1 - 0.1823215).abs() < 1e-6);
}
