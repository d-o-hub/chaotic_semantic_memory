use super::*;

#[test]
fn test_normalize_scores_range_two() {
    let scores = vec![("a".to_string(), 2.0), ("b".to_string(), 0.0)];
    let normalized = normalize_scores(&scores);
    // (2-0)/2 = 1.0; (0-0)/2 = 0.0
    assert!((normalized[0].1 - 1.0).abs() < 1e-6);
    assert!((normalized[1].1 - 0.0).abs() < 1e-6);
}

#[test]
fn test_normalize_scores_empty() {
    let normalized = normalize_scores(&[]);
    assert!(normalized.is_empty());
}

#[test]
fn test_normalize_scores_equal() {
    let scores = vec![("a".to_string(), 5.0), ("b".to_string(), 5.0)];
    let normalized = normalize_scores(&scores);

    // All equal scores should normalize to 1.0
    assert!((normalized[0].1 - 1.0).abs() < 1e-6);
    assert!((normalized[1].1 - 1.0).abs() < 1e-6);
}

#[test]
fn test_merge_results_basic() {
    let bm25 = vec![("doc1".to_string(), 1.0), ("doc2".to_string(), 0.5)];
    let hdc = vec![("doc1".to_string(), 0.5), ("doc3".to_string(), 1.0)];

    let merged = merge_results(&bm25, &hdc, (0.5, 0.5));

    // doc1 appears in both
    assert!(merged.iter().any(|(id, _)| id == "doc1"));
    // doc2 only in BM25
    assert!(merged.iter().any(|(id, _)| id == "doc2"));
    // doc3 only in HDC
    assert!(merged.iter().any(|(id, _)| id == "doc3"));
}

#[test]
fn test_merge_results_weighted() {
    let bm25 = vec![("doc1".to_string(), 1.0)];
    let hdc = vec![("doc1".to_string(), 1.0)];

    // With heavy keyword weight, BM25 should dominate
    let merged = merge_results(&bm25, &hdc, (0.9, 0.1));

    // doc1 should have combined score
    assert!(merged.iter().any(|(id, s)| id == "doc1" && *s > 0.0));
}

#[test]
fn test_merge_results_empty() {
    let merged = merge_results(&[], &[], (0.5, 0.5));
    assert!(merged.is_empty());

    let merged = merge_results(&[("a".to_string(), 1.0)], &[], (0.5, 0.5));
    assert_eq!(merged.len(), 1);

    let merged = merge_results(&[], &[("a".to_string(), 1.0)], (0.5, 0.5));
    assert_eq!(merged.len(), 1);
}

#[test]
fn test_exact_score_calculation() {
    // Use non-zero min values to catch replace - with + mutants.
    // Use weight != 0.5 to catch weight-related mutants.
    let weights = (0.6, 0.4);
    let bm25 = vec![
        ("d1".to_string(), 12.0),
        ("d2".to_string(), 2.0),
        ("d4".to_string(), 7.0),
    ];
    let hdc = vec![
        ("d1".to_string(), 1.2),
        ("d3".to_string(), 0.2),
        ("d4".to_string(), 0.7),
    ];

    let merged = merge_results(&bm25, &hdc, weights);

    // d1: 0.6 * 1.0 + 0.4 * 1.0 = 1.0
    let d1_score = merged.iter().find(|(id, _)| id == "d1").unwrap().1;
    assert!((d1_score - 1.0).abs() < 1e-6);

    // d2: 0.6 * 0.0 + 0.0 = 0.0
    let d2_score = merged.iter().find(|(id, _)| id == "d2").unwrap().1;
    assert!((d2_score - 0.0).abs() < 1e-6);

    // d3: 0.0 + 0.4 * 0.0 = 0.0
    let d3_score = merged.iter().find(|(id, _)| id == "d3").unwrap().1;
    assert!((d3_score - 0.0).abs() < 1e-6);

    // d4: 0.6 * 0.5 + 0.4 * 0.5 = 0.5
    let d4_score = merged.iter().find(|(id, _)| id == "d4").unwrap().1;
    assert!((d4_score - 0.5).abs() < 1e-6);
}

#[test]
fn test_merge_results_equal_scores() {
    let bm25 = vec![("d1".to_string(), 5.0), ("d2".to_string(), 5.0)];
    let hdc = vec![("d1".to_string(), 0.5), ("d2".to_string(), 0.5)];
    let weights = (0.5, 0.5);
    let merged = merge_results(&bm25, &hdc, weights);
    // Each gets kw_weight (0.5) + sem_weight (0.5) = 1.0
    assert_eq!(merged.len(), 2);
    for (_, score) in merged {
        assert!((score - 1.0).abs() < 1e-6);
    }
}

#[test]
fn test_range_epsilon_boundary() {
    let epsilon = 1e-10;
    let scores = vec![("a".to_string(), epsilon), ("b".to_string(), 0.0)];
    let normalized = normalize_scores(&scores);
    // range = epsilon. epsilon < epsilon is false.
    assert!((normalized[0].1 - 1.0).abs() < 1e-6);
    assert!((normalized[1].1 - 0.0).abs() < 1e-6);

    let just_below = epsilon * 0.9;
    let scores_below = vec![("a".to_string(), just_below), ("b".to_string(), 0.0)];
    let normalized_below = normalize_scores(&scores_below);
    // range < epsilon is true.
    assert!((normalized_below[0].1 - 1.0).abs() < 1e-6);
    assert!((normalized_below[1].1 - 1.0).abs() < 1e-6);
}

#[test]
fn test_merge_results_epsilon_boundary() {
    let epsilon = 1e-10;
    let weights = (0.5, 0.5);

    // Case 1: range exactly epsilon (should normalize)
    let bm25 = vec![("d1".to_string(), epsilon), ("d2".to_string(), 0.0)];
    let hdc = vec![("d1".to_string(), epsilon), ("d2".to_string(), 0.0)];
    let merged = merge_results(&bm25, &hdc, weights);
    let d1_score = merged.iter().find(|(id, _)| id == "d1").unwrap().1;
    let d2_score = merged.iter().find(|(id, _)| id == "d2").unwrap().1;
    assert!((d1_score - 1.0).abs() < 1e-6);
    assert!((d2_score - 0.0).abs() < 1e-6);

    // Case 2: range just below epsilon (should fallback to 1.0)
    let just_below = epsilon * 0.9;
    let bm25_small = vec![("d1".to_string(), just_below), ("d2".to_string(), 0.0)];
    let hdc_small = vec![("d1".to_string(), just_below), ("d2".to_string(), 0.0)];
    let merged_small = merge_results(&bm25_small, &hdc_small, weights);
    for (_, score) in merged_small {
        assert!((score - 1.0).abs() < 1e-6);
    }
}

fn config_with_threshold(min_score: f32) -> HybridConfig {
    HybridConfig {
        mode: HybridMode::Auto,
        min_score,
    }
}

#[test]
fn test_hits_above_threshold() {
    let bm25 = vec![("doc_a".to_string(), 0.9)];
    let hdc = vec![("doc_a".to_string(), 0.8)];
    let config = config_with_threshold(0.5);
    let weights = (0.6, 0.4);
    let result = merge_results_checked(&bm25, &hdc, weights, &config, "test query");
    assert!(matches!(result, HybridResult::Hits(_)));
}

#[test]
fn test_abstention_below_threshold() {
    let bm25 = vec![("doc_a".to_string(), 0.1)];
    let hdc = vec![("doc_a".to_string(), 0.05)];
    let config = config_with_threshold(0.5);
    let weights = (0.6, 0.4);
    let result = merge_results_checked(&bm25, &hdc, weights, &config, "unknown concept");
    match result {
        HybridResult::Abstained(a) => {
            assert_eq!(a.query, "unknown concept");
            assert!(a.best_score_seen < 0.5);
            assert!((a.min_score_threshold - 0.5).abs() < 1e-6);
        }
        HybridResult::Hits(_) => panic!("Expected abstention"),
    }
}

#[test]
fn test_empty_results_produce_abstention() {
    let bm25: Vec<(String, f32)> = vec![];
    let hdc: Vec<(String, f32)> = vec![];
    let config = config_with_threshold(0.3);
    let weights = (0.5, 0.5);
    let result = merge_results_checked(&bm25, &hdc, weights, &config, "empty corpus query");
    assert!(matches!(result, HybridResult::Abstained(_)));
}

#[test]
fn test_abstention_best_score_is_highest_seen() {
    let bm25 = vec![("doc_a".to_string(), 0.3), ("doc_b".to_string(), 0.1)];
    let hdc = vec![("doc_a".to_string(), 0.2)];
    let config = config_with_threshold(0.5);
    let weights = (0.6, 0.4);
    let result = merge_results_checked(&bm25, &hdc, weights, &config, "test");
    if let HybridResult::Abstained(a) = result {
        assert!(a.best_score_seen > 0.0 && a.best_score_seen < 0.5);
    } else {
        panic!("Expected abstention");
    }
}

#[test]
fn test_merge_results_checked_exact_score_threshold() {
    // Catch replace * with + or / mutants in merge_results_checked
    // Use values where (w * s) is distinct from (w + s) and (w / s)
    // Weight = 0.5, Score = 0.8 => w * s = 0.4, w + s = 1.3, w / s = 0.625
    let bm25 = vec![("doc_a".to_string(), 0.8)];
    let weights = (0.5, 0.5);

    // Case 1: Threshold = 0.5. (0.5 * 0.8 = 0.4) < 0.5 => Abstained
    let config_abstain = config_with_threshold(0.5);
    let result = merge_results_checked(&bm25, &[], weights, &config_abstain, "test");
    if let HybridResult::Abstained(a) = result {
        assert!(
            (a.best_score_seen - 0.4).abs() < 1e-6,
            "Expected best_score_seen to be 0.4, got {}",
            a.best_score_seen
        );
    } else {
        panic!("Expected abstention for score 0.4 with threshold 0.5");
    }

    // Case 2: Threshold = 0.3. (0.5 * 0.8 = 0.4) >= 0.3 => Hits
    let config_hits = config_with_threshold(0.3);
    let result = merge_results_checked(&bm25, &[], weights, &config_hits, "test");
    assert!(
        matches!(result, HybridResult::Hits(_)),
        "Expected Hits for score 0.4 with threshold 0.3"
    );
}

#[test]
fn test_merge_results_checked_hdc_path() {
    let hdc = vec![("doc_a".to_string(), 0.8)];
    let weights = (0.5, 0.5);
    let config_abstain = config_with_threshold(0.5);
    let result = merge_results_checked(&[], &hdc, weights, &config_abstain, "test");
    if let HybridResult::Abstained(a) = result {
        assert!((a.best_score_seen - 0.4).abs() < 1e-6);
    } else {
        panic!("Expected abstention for HDC score 0.4 with threshold 0.5");
    }
}

#[test]
fn test_merge_results_checked_combined_exact_score() {
    let bm25 = vec![("doc_a".to_string(), 0.6)]; // kw_weight * 0.6
    let hdc = vec![("doc_a".to_string(), 0.4)]; // sem_weight * 0.4
    let weights = (0.7, 0.3);
    // expected = 0.7 * 0.6 + 0.3 * 0.4 = 0.42 + 0.12 = 0.54

    let config = config_with_threshold(0.6);
    let result = merge_results_checked(&bm25, &hdc, weights, &config, "test");
    if let HybridResult::Abstained(a) = result {
        assert!(
            (a.best_score_seen - 0.54).abs() < 1e-6,
            "Expected 0.54, got {}",
            a.best_score_seen
        );
    } else {
        panic!("Expected abstention for 0.54 < 0.6");
    }
}
