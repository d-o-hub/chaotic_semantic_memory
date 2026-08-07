use super::*;

#[test]
fn test_compute_weights() {
    let cases = vec![(1, 0.9, 0.1), (3, 0.7, 0.3), (5, 0.4, 0.6), (9, 0.2, 0.8)];
    for (tc, expected_kw, expected_sem) in cases {
        let (kw, sem) = compute_weights(tc);
        assert!((kw - expected_kw).abs() < 1e-6);
        assert!((sem - expected_sem).abs() < 1e-6);
    }
}

#[test]
fn test_normalize_scores() {
    let scores = vec![
        ("a".to_string(), 10.0),
        ("b".to_string(), 15.0),
        ("c".to_string(), 20.0),
    ];
    let normalized = normalize_scores(&scores);
    assert!((normalized[0].1 - 0.0).abs() < 1e-6);
    assert!((normalized[1].1 - 0.5).abs() < 1e-6);
    assert!((normalized[2].1 - 1.0).abs() < 1e-6);

    let scores = vec![("a".to_string(), 2.0), ("b".to_string(), 0.0)];
    let normalized = normalize_scores(&scores);
    assert!((normalized[0].1 - 1.0).abs() < 1e-6);
    assert!((normalized[1].1 - 0.0).abs() < 1e-6);

    assert!(normalize_scores(&[]).is_empty());

    let scores = vec![("a".to_string(), 5.0), ("b".to_string(), 5.0)];
    let normalized = normalize_scores(&scores);
    assert!((normalized[0].1 - 1.0).abs() < 1e-6);
    assert!((normalized[1].1 - 1.0).abs() < 1e-6);
}

#[test]
fn test_normalize_scores_in_place_parity() {
    let mut empty = Vec::new();
    normalize_scores_in_place(&mut empty);
    assert!(empty.is_empty());
    let mut single = vec![("a".to_string(), 10.0)];
    normalize_scores_in_place(&mut single);
    assert!((single[0].1 - 1.0).abs() < 1e-6);

    let cases = vec![
        ("a".to_string(), 10.0),
        ("b".to_string(), 15.0),
        ("c".to_string(), 20.0),
    ];
    let mut multi = cases.clone();
    normalize_scores_in_place(&mut multi);
    let expected = normalize_scores(&cases);
    assert_eq!(multi.len(), expected.len());
    for i in 0..multi.len() {
        assert_eq!(multi[i].0, expected[i].0);
        assert!((multi[i].1 - expected[i].1).abs() < 1e-6);
    }
}

#[test]
fn test_merge_results() {
    let bm25 = vec![("doc1".to_string(), 1.0), ("doc2".to_string(), 0.5)];
    let hdc = vec![("doc1".to_string(), 0.5), ("doc3".to_string(), 1.0)];
    let merged = merge_results(&bm25, &hdc, (0.5, 0.5), 10);
    assert!(merged.iter().any(|(id, _)| id == "doc1"));
    assert!(merged.iter().any(|(id, _)| id == "doc2"));
    assert!(merged.iter().any(|(id, _)| id == "doc3"));

    let bm25 = vec![("doc1".to_string(), 1.0)];
    let hdc = vec![("doc1".to_string(), 1.0)];
    let merged = merge_results(&bm25, &hdc, (0.9, 0.1), 10);
    assert!(merged.iter().any(|(id, s)| id == "doc1" && *s > 0.0));

    assert!(merge_results(&[], &[], (0.5, 0.5), 10).is_empty());
    assert_eq!(
        merge_results(&[("a".to_string(), 1.0)], &[], (0.5, 0.5), 10).len(),
        1
    );
    assert_eq!(
        merge_results(&[], &[("a".to_string(), 1.0)], (0.5, 0.5), 10).len(),
        1
    );
}

#[test]
fn test_exact_score_calculation() {
    let weights = (0.6, 0.4);
    let bm25 = vec![("d1".into(), 12.0), ("d2".into(), 2.0), ("d4".into(), 7.0)];
    let hdc = vec![("d1".into(), 1.2), ("d3".into(), 0.2), ("d4".into(), 0.7)];
    let merged = merge_results(&bm25, &hdc, weights, 10);

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
fn test_range_epsilon_boundary() {
    let epsilon = f32::EPSILON;
    let norm = normalize_scores(&[("a".into(), epsilon), ("b".into(), 0.0)]);
    // range = epsilon. epsilon < epsilon is false.
    assert!((norm[0].1 - 1.0).abs() < 1e-6);
    assert!((norm[1].1 - 0.0).abs() < 1e-6);

    let norm_below = normalize_scores(&[("a".into(), epsilon * 0.9), ("b".into(), 0.0)]);
    // range < epsilon is true.
    assert!((norm_below[0].1 - 1.0).abs() < 1e-6);
    assert!((norm_below[1].1 - 1.0).abs() < 1e-6);
}

#[test]
fn test_merge_results_epsilon_boundary() {
    let epsilon = f32::EPSILON;
    let weights = (0.5, 0.5);
    let bm25 = vec![("d1".into(), epsilon), ("d2".into(), 0.0)];
    let hdc = vec![("d1".into(), epsilon), ("d2".into(), 0.0)];
    let merged = merge_results(&bm25, &hdc, weights, 10);
    let d1_score = merged.iter().find(|(id, _)| id == "d1").unwrap().1;
    let d2_score = merged.iter().find(|(id, _)| id == "d2").unwrap().1;
    assert!((d1_score - 1.0).abs() < 1e-6);
    assert!((d2_score - 0.0).abs() < 1e-6);

    let bm25_s = vec![("d1".into(), epsilon * 0.9), ("d2".into(), 0.0)];
    let hdc_s = vec![("d1".into(), epsilon * 0.9), ("d2".into(), 0.0)];
    let merged_s = merge_results(&bm25_s, &hdc_s, weights, 10);
    for (_, score) in merged_s {
        assert!((score - 1.0).abs() < 1e-6);
    }
}

#[test]
fn test_merge_results_top_k() {
    let bm25 = vec![
        ("d1".to_string(), 10.0),
        ("d2".to_string(), 8.0),
        ("d3".to_string(), 6.0),
    ];
    let hdc = vec![
        ("d1".to_string(), 10.0),
        ("d4".to_string(), 4.0),
        ("d5".to_string(), 2.0),
    ];
    let weights = (0.5, 0.5);

    // top_k = 2 should return exactly the 2 best elements: d1 and d2
    let merged = merge_results(&bm25, &hdc, weights, 2);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].0, "d1");
    assert_eq!(merged[1].0, "d2");

    // top_k = 1 should return only d1
    let merged_one = merge_results(&bm25, &hdc, weights, 1);
    assert_eq!(merged_one.len(), 1);
    assert_eq!(merged_one[0].0, "d1");

    // top_k = 0 should return empty
    let merged_zero = merge_results(&bm25, &hdc, weights, 0);
    assert!(merged_zero.is_empty());
}

#[test]
fn test_merge_results_top_k_exact_boundary() {
    // When unique result count equals top_k, the partial-sort branch must NOT run.
    // Using `>=` instead of `>` would call select_nth_unstable_by(top_k) with
    // index == len and panic.
    let bm25 = vec![("d1".to_string(), 10.0), ("d2".to_string(), 8.0)];
    let hdc = vec![("d1".to_string(), 10.0), ("d2".to_string(), 8.0)];
    let weights = (0.5, 0.5);
    let merged = merge_results(&bm25, &hdc, weights, 2);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].0, "d1");
    assert_eq!(merged[1].0, "d2");
}

#[test]
fn test_single_list_exact() {
    let bm25 = vec![("d1".into(), 12.0), ("d2".into(), 2.0), ("d4".into(), 7.0)];
    let merged = merge_results(&bm25, &[], (0.6, 0.4), 10);
    assert_eq!(merged.len(), 3);
    assert!((merged[0].1 - 0.6).abs() < 1e-6);
    assert!((merged[1].1 - 0.3).abs() < 1e-6);
    assert!((merged[2].1 - 0.0).abs() < 1e-6);
}

#[test]
fn test_single_list_equal() {
    let bm25 = vec![("d1".into(), 5.0), ("d2".into(), 5.0)];
    let merged = merge_results(&bm25, &[], (0.5, 0.5), 10);
    assert_eq!(merged.len(), 2);
    assert!((merged[0].1 - 0.5).abs() < 1e-6);
    assert!((merged[1].1 - 0.5).abs() < 1e-6);
}

#[test]
fn test_single_list_top_k() {
    let bm25 = vec![("d1".into(), 10.0), ("d2".into(), 8.0), ("d3".into(), 6.0)];
    let merged = merge_results(&bm25, &[], (0.5, 0.5), 2);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].0, "d1");
    assert_eq!(merged[1].0, "d2");

    let merged_boundary = merge_results(&bm25, &[], (0.5, 0.5), 3);
    assert_eq!(merged_boundary.len(), 3);

    assert!(merge_results(&bm25, &[], (0.5, 0.5), 0).is_empty());
}

#[test]
fn test_single_list_epsilon() {
    let epsilon = f32::EPSILON;
    let bm25 = vec![("d1".into(), epsilon), ("d2".into(), 0.0)];
    let merged = merge_results(&bm25, &[], (0.5, 0.5), 10);
    assert!((merged[0].1 - 0.5).abs() < 1e-6);
    assert!((merged[1].1 - 0.0).abs() < 1e-6);

    let bm25_below = vec![("d1".into(), epsilon * 0.9), ("d2".into(), 0.0)];
    let merged_below = merge_results(&bm25_below, &[], (0.5, 0.5), 10);
    assert!((merged_below[0].1 - 0.5).abs() < 1e-6);
    assert!((merged_below[1].1 - 0.5).abs() < 1e-6);
}
