#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
fn test_top_k_edge_cases() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world"]);
    index.add_document("doc2", &["hello", "rust"]);

    // top_k == 0 returns empty without scoring
    let results = index.search(&["hello"], 0);
    assert!(results.is_empty());

    // top_k == usize::MAX is clamped to MAX_TOP_K_LIMIT (CWE-770)
    let results = index.search(&["hello"], usize::MAX);
    assert_eq!(results.len(), 2);
}

#[test]
fn test_idf_rare_term_higher_score() {
    let mut index = Bm25Index::new();

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

    // Warm-cache parity (use miri -Zmiri-deterministic-floats in CI).
    let results3 = index.search(&["hello"], 10);
    assert_eq!(results2.len(), results3.len());
    for ((id_a, s_a), (id_b, s_b)) in results2.iter().zip(results3.iter()) {
        assert_eq!(id_a, id_b);
        assert!((s_a - s_b).abs() < 1e-6, "{id_a}: {s_a} vs {s_b}");
    }
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
fn test_oov_term_excluded_from_scoring() {
    let mut index = Bm25Index::new();
    index.add_document("doc1", &["hello", "world"]);
    index.add_document("doc2", &["hello", "rust"]);

    let results = index.search(&["hello", "nonexistent_term_xyz"], 10);
    assert_eq!(results.len(), 2, "both docs with 'hello' must be returned");

    let score_no_oov = index.search(&["hello"], 10);
    for (id, score) in &results {
        let expected = score_no_oov.iter().find(|(i, _)| i == id).unwrap().1;
        assert!(
            (*score - expected).abs() < 1e-6,
            "OOV term must not affect score of '{id}': got {score}, expected {expected}"
        );
    }
}

#[test]
fn test_idf_formula_positive_for_all_valid_inputs() {
    let mut index = Bm25Index::new();
    for i in 0..20 {
        index.add_document(&format!("doc{i}"), &["term_a"]);
    }

    let results = index.search(&["term_a"], 20);
    assert_eq!(results.len(), 20);

    let n = 20.0f32;
    let df = 20.0f32;
    let expected_idf = ((n + 1.0) / (df + 0.5)).ln();
    assert!(expected_idf > 0.0, "IDF must be positive for df=N case");

    for (_, score) in &results {
        assert!(*score > 0.0, "score must be positive when IDF > 0");
    }
}

#[test]
fn test_rare_term_scores_higher_than_common_term() {
    let mut index = Bm25Index::new();
    index.add_document("common-only", &["common"]);
    index.add_document("rare-and-common", &["rare", "common"]);

    let results_both = index.search(&["rare", "common"], 10);
    let results_common = index.search(&["common"], 10);

    let score_both = results_both
        .iter()
        .find(|(id, _)| id == "rare-and-common")
        .unwrap()
        .1;
    let score_common = results_common
        .iter()
        .find(|(id, _)| id == "rare-and-common")
        .unwrap()
        .1;

    assert!(
        score_both > score_common,
        "adding a rare term must increase score: both={score_both} > common={score_common}"
    );
}

#[test]
fn test_score_positive_for_single_doc_single_term() {
    let mut index = Bm25Index::new();
    index.add_document("solo", &["unique"]);

    let results = index.search(&["unique"], 10);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].1 > 0.0,
        "single doc with matching term must score > 0, got {}",
        results[0].1
    );
}

#[test]
fn test_search_does_not_mutate_index() {
    let mut index = Bm25Index::new();
    index.add_document("d1", &["alpha", "beta"]);
    index.add_document("d2", &["alpha", "gamma"]);

    let before_len = index.len();
    let _ = index.search(&["alpha"], 10);
    let _ = index.search(&["beta"], 10);
    assert_eq!(
        index.len(),
        before_len,
        "search must not change document count"
    );

    let results_after = index.search(&["alpha"], 10);
    assert_eq!(
        results_after.len(),
        2,
        "index still queryable after searches"
    );
}

// Regression: every distinct query term must contribute to scoring.
// Guards short-query (<= 8 tokens) linear-scan dedup (PR #363, bm25.rs:271-283).
#[test]
fn test_search_distinct_terms_each_contribute() {
    let mut index = Bm25Index::new();
    index.add_document("doc_alpha", &["alpha", "alpha", "alpha"]);
    index.add_document("doc_beta", &["beta", "beta", "beta"]);
    let results = index.search(&["alpha", "beta"], 10);
    assert_eq!(results.len(), 2, "both distinct terms must score their doc");
    let ids: std::collections::HashSet<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains("doc_alpha"));
    assert!(ids.contains("doc_beta"));
    assert!(results.iter().all(|(_, score)| *score > 0.0));
}
// Companion: same distinct-term invariant on HashSet dedup path (> 8 tokens).
#[test]
fn test_search_dedup_hashset_path_distinct_terms() {
    let mut index = Bm25Index::new();
    index.add_document("doc_a", &["a"]);
    index.add_document("doc_b", &["b"]);

    let query = ["a", "a", "c", "d", "e", "f", "g", "h", "i", "b"];
    let results = index.search(&query, 10);
    let ids: std::collections::HashSet<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains("doc_a"));
    assert!(ids.contains("doc_b"));

    // Duplicate terms must not inflate score
    let once = index.search(&["a"], 10);
    let twice = index.search(&["a", "a"], 10);
    assert!(
        (once[0].1 - twice[0].1).abs() < 1e-6,
        "dupes must not inflate score"
    );
}

// Kills mutation: `replace > with >= in Bm25Index::search` (score > 0.0 threshold).
// Documents with no shared terms have score == 0.0 and must be excluded.
#[test]
fn test_zero_score_documents_excluded_from_results() {
    let mut index = Bm25Index::new();
    index.add_document("match1", &["target", "extra"]);
    index.add_document("match2", &["target"]);
    index.add_document("nomatch1", &["unrelated", "words"]);
    index.add_document("nomatch2", &["other", "content"]);
    index.add_document("nomatch3", &["completely", "different"]);

    let results = index.search(&["target"], 100);

    assert_eq!(results.len(), 2, "only score > 0.0 docs returned");
    for (id, score) in &results {
        assert!(*score > 0.0, "doc '{id}' has non-positive score {score}");
    }
    let ids: std::collections::HashSet<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(!ids.contains("nomatch1"));
    assert!(!ids.contains("nomatch2"));
    assert!(!ids.contains("nomatch3"));
}
