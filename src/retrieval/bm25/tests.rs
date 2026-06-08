// Exact float comparisons for BM25 score test assertions

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
    index.add_document("doc3", &["hello", "python"]);

    // Removing "doc1" triggers swap_remove, doc3 moves to index 0
    index.remove_document("doc1");
    assert_eq!(index.len(), 2);

    // Verify removed doc is gone
    let results = index.search(&["world"], 10);
    assert!(results.is_empty());

    // Verify swapped doc is still findable (this catches the postings index update mutation)
    let results = index.search(&["python"], 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "doc3");

    // Verify common term still works for both
    let results = index.search(&["hello"], 10);
    assert_eq!(results.len(), 2);
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
    // Long document with same term but more other words
    index.add_document(
        "long",
        &[
            "hello", "other", "words", "here", "and", "even", "more", "words",
        ],
    );

    // Both match, but shorter doc should score higher per-term
    // (BM25 normalizes by document length)
    let results = index.search(&["hello"], 10);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "short");
}

#[test]
fn test_scoring_impact_tf() {
    let mut index = Bm25Index::new();
    // Same length docs
    index.add_document("doc1", &["hello", "a", "b", "c"]);
    index.add_document("doc2", &["hello", "hello", "a", "b"]);

    let results = index.search(&["hello"], 10);
    assert_eq!(results[0].0, "doc2");
    assert!(results[0].1 > results[1].1);
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
fn test_exact_score_calculation() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello"]);

    let results = index.search(&["hello"], 10);
    assert_eq!(results.len(), 1);

    // For N=1, df=1, tf=1, dl=avgdl:
    // score = ln((N+1)/(df+0.5)) = ln(2/1.5)
    let expected = (2.0f32 / 1.5f32).ln();
    assert!((results[0].1 - expected).abs() < 1e-6);
}

#[test]
fn test_internal_alignment() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["a", "b"]);
    index.add_document("doc2", &["a", "b", "c"]);
    index.add_document("doc3", &["a"]);

    assert_eq!(index.documents.len(), 3);
    assert_eq!(index.doc_lengths.len(), 3);
    assert!((index.doc_lengths[0] - 2.0).abs() < f32::EPSILON);
    assert!((index.doc_lengths[1] - 3.0).abs() < f32::EPSILON);
    assert!((index.doc_lengths[2] - 1.0).abs() < f32::EPSILON);

    // Swap remove doc1 (index 0). doc3 (index 2) should move to index 0.
    index.remove_document("doc1");
    assert_eq!(index.documents.len(), 2);
    assert_eq!(index.doc_lengths.len(), 2);
    assert_eq!(index.documents[0].id, "doc3");
    assert!((index.doc_lengths[0] - 1.0).abs() < f32::EPSILON);
    assert_eq!(index.documents[1].id, "doc2");
    assert!((index.doc_lengths[1] - 3.0).abs() < f32::EPSILON);

    index.clear();
    assert!(index.doc_lengths.is_empty());
}

#[test]
fn test_cache_consistency() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello"]);
    index.add_document("doc2", &["hello", "hello"]);

    // Initial search populates cache
    let results = index.search(&["hello"], 10);
    assert_eq!(results.len(), 2);
    let score1_initial = results.iter().find(|(id, _)| id == "doc1").unwrap().1;

    // Mutation invalidates cache
    index.add_document("doc3", &["world"]);

    // Second search recomputes cache
    let results2 = index.search(&["hello"], 10);
    let score1_after = results2.iter().find(|(id, _)| id == "doc1").unwrap().1;

    // Scores should change because avgdl changed
    assert!((score1_initial - score1_after).abs() > 1e-6);

    // Identity check for cached search
    let results3 = index.search(&["hello"], 10);
    let score1_cached = results3.iter().find(|(id, _)| id == "doc1").unwrap().1;
    assert!((score1_after - score1_cached).abs() < f32::EPSILON);
}

#[test]
fn test_clone_preserves_state() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["a"]);

    let cloned = index.clone();
    assert_eq!(cloned.len(), 1);
    assert_eq!(cloned.search(&["a"], 10).len(), 1);

    // Mutation on original doesn't affect clone
    index.add_document("doc2", &["b"]);
    assert_eq!(index.len(), 2);
    assert_eq!(cloned.len(), 1);
}

#[test]
fn test_scoring_math_general_case() {
    let mut index = Bm25Index::new();
    // doc1: "a" (tf=2), len=2
    index.add_document("doc1", &["a", "a"]);
    // doc2: "a" (tf=1), len=1
    index.add_document("doc2", &["a"]);

    let results = index.search(&["a"], 10);
    assert_eq!(results.len(), 2);

    let n = 2.0f32;
    let df = 2.0f32;
    let avgdl = 1.5f32;
    let k1 = 1.2f32;
    let b = 0.75f32;

    let idf = ((n + 1.0) / (df + 0.5)).ln();
    let weighted_idf = idf * (k1 + 1.0);

    // doc2: tf=1, len=1
    let b_doc2 = k1 * (1.0 - b) + (k1 * b / avgdl) * 1.0;
    let expected_doc2 = weighted_idf / (1.0 + b_doc2);
    let score_doc2 = results.iter().find(|(id, _)| id == "doc2").unwrap().1;
    assert!((score_doc2 - expected_doc2).abs() < 1e-6);

    // doc1: tf=2, len=2
    let b_doc1 = k1 * (1.0 - b) + (k1 * b / avgdl) * 2.0;
    let expected_doc1 = (2.0 * weighted_idf) / (2.0 + b_doc1);
    let score_doc1 = results.iter().find(|(id, _)| id == "doc1").unwrap().1;
    assert!((score_doc1 - expected_doc1).abs() < 1e-6);
}

#[test]
fn test_search_mutant_prevention() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["a"]);

    // query_tokens is empty but index is NOT empty, top_k > 0
    // If || was replaced with &&, this would NOT return empty
    assert!(index.search::<&str>(&[], 10).is_empty());

    // index is empty but query_tokens is NOT empty, top_k > 0
    let index_empty = Bm25Index::new();
    assert!(index_empty.search(&["a"], 10).is_empty());
}

#[test]
fn test_search_duplicate_tokens() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world"]);

    // Duplicated token "hello" should yield same score as single "hello"
    let results1 = index.search(&["hello"], 10);
    let results2 = index.search(&["hello", "hello"], 10);

    assert_eq!(results1.len(), results2.len());
    assert!((results1[0].1 - results2[0].1).abs() < f32::EPSILON);
}

#[test]
fn test_search_idf_threshold() {
    let mut index = Bm25Index::new();
    // Term "common" appears in all 3 documents
    index.add_document("doc1", &["common", "rare"]);
    index.add_document("doc2", &["common"]);
    index.add_document("doc3", &["common"]);

    // N=3, df=3. idf = ln((3+1)/(3+0.5)) = ln(4/3.5) = ln(1.14) > 0
    let results = index.search(&["common"], 10);
    assert_eq!(results.len(), 3);

    // Add many more documents containing "common" until IDF <= 0
    for i in 4..20 {
        index.add_document(&format!("doc{}", i), &["common"]);
    }

    // N=19, df=19. idf = ln((19+1)/(19+0.5)) = ln(20/19.5) = ln(1.02) > 0
    // We need df > N/2 + offset for IDF to become negative?
    // No, idf = ln((N+1)/(df+0.5)). If df+0.5 > N+1, idf < 0.
    // df > N + 0.5. Since df <= N, idf is always > 0 with this formula.
    // Wait, Okapi BM25 typically uses idf = ln((N - df + 0.5) / (df + 0.5)).
    // But our implementation uses: idf = ((n + 1.0) / (df as f32 + 0.5)).ln();
    // This formula ALWAYS gives idf > 0 since n >= df.

    let results_common = index.search(&["common"], 10);
    assert!(!results_common.is_empty());
}
