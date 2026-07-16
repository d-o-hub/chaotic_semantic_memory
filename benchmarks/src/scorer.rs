use crate::types::{CaseResult, QueryCase};
use std::collections::HashSet;

/// Hit rate@k: true if any gold evidence ID appears in the top-k retrieved items.
pub fn hit_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> bool {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    result
        .retrieved
        .iter()
        .take(k)
        .any(|r| gold.contains(r.memory_id.as_str()))
}

/// Multi-label recall@k: |unique gold ∩ retrieved@k| / |unique gold|.
/// Returns 0.0 when gold is empty.
pub fn recall_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> f32 {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    if gold.is_empty() {
        return 0.0;
    }
    let retrieved: HashSet<&str> = result
        .retrieved
        .iter()
        .take(k)
        .map(|r| r.memory_id.as_str())
        .collect();
    let hits = gold.intersection(&retrieved).count();
    hits as f32 / gold.len() as f32
}

pub fn reciprocal_rank(case: &QueryCase, result: &CaseResult) -> f32 {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    for item in &result.retrieved {
        if gold.contains(item.memory_id.as_str()) {
            return 1.0 / item.rank as f32;
        }
    }
    0.0
}

/// Logarithmic position discount for 0-indexed list position `i`.
/// Equals `1 / log2(rank + 1)` where rank is 1-indexed (`rank = i + 1`).
#[inline]
fn log_discount(i: usize) -> f32 {
    1.0 / ((i as f32 + 2.0).log2())
}

/// Compute Normalized Discounted Cumulative Gain at k.
///
/// Uses true logarithmic discount: relevance / log2(rank + 1) for 1-indexed rank
/// (equivalently `1.0 / ((i as f32 + 2.0).log2())` for 0-indexed position `i`).
/// For binary relevance (gold/not-gold), this rewards systems that return relevant
/// items higher in the result list.
pub fn ndcg_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> f32 {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    if gold.is_empty() {
        return 0.0;
    }

    // DCG: sum of log-discount for each relevant item in the top-k ranking
    let dcg: f32 = result
        .retrieved
        .iter()
        .take(k)
        .enumerate()
        .filter(|(_, r)| gold.contains(r.memory_id.as_str()))
        .map(|(i, _)| log_discount(i))
        .sum();

    // Ideal DCG: all unique gold items ranked at the top (up to k)
    let ideal_count = gold.len().min(k);
    let ideal_dcg: f32 = (0..ideal_count).map(log_discount).sum();

    if ideal_dcg > 0.0 {
        dcg / ideal_dcg
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RetrievedItem, TaskType};

    fn make_query_case(gold_ids: Vec<&str>) -> QueryCase {
        QueryCase {
            query_id: "test-query".to_string(),
            session_id: "test-session".to_string(),
            task_type: TaskType::Recall,
            query: "test query".to_string(),
            gold_evidence_ids: gold_ids.iter().map(|s| s.to_string()).collect(),
            expected_answer: None,
            should_abstain: false,
        }
    }

    fn make_case_result(retrieved: Vec<(&str, usize, f32)>) -> CaseResult {
        CaseResult {
            query_id: "test-query".to_string(),
            session_id: "test-session".to_string(),
            task_type: TaskType::Recall,
            retrieved: retrieved
                .iter()
                .map(|(id, rank, score)| RetrievedItem {
                    memory_id: id.to_string(),
                    rank: *rank,
                    score: *score,
                })
                .collect(),
            recall_at_1: 0.0,
            recall_at_5: 0.0,
            recall_at_10: 0.0,
            reciprocal_rank: 0.0,
            ndcg_at_10: 0.0,
            predicted_answer: None,
            exact_match: None,
            abstained: false,
            should_abstain: false,
            latency_ms: 0,
            latency_us: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
        }
    }

    #[test]
    fn test_hit_at_k_gold_at_rank_1() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9), ("other", 2, 0.7)]);
        assert!(hit_at_k(&case, &result, 1));
        assert!(hit_at_k(&case, &result, 5));
        assert!(hit_at_k(&case, &result, 10));
    }

    #[test]
    fn test_hit_at_k_gold_at_rank_3() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![
            ("other-1", 1, 0.9),
            ("other-2", 2, 0.8),
            ("gold-1", 3, 0.7),
        ]);
        assert!(!hit_at_k(&case, &result, 1));
        assert!(!hit_at_k(&case, &result, 2));
        assert!(hit_at_k(&case, &result, 3));
        assert!(hit_at_k(&case, &result, 5));
    }

    #[test]
    fn test_hit_at_k_no_match() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("other-1", 1, 0.9), ("other-2", 2, 0.8)]);
        assert!(!hit_at_k(&case, &result, 1));
        assert!(!hit_at_k(&case, &result, 5));
        assert!(!hit_at_k(&case, &result, 10));
    }

    #[test]
    fn test_hit_at_k_multiple_gold_ids() {
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![("gold-2", 1, 0.9), ("other", 2, 0.7)]);
        assert!(hit_at_k(&case, &result, 1)); // gold-2 matches
    }

    #[test]
    fn test_recall_at_k_multi_gold_partial() {
        // 2 of 3 gold in top-5 → 2/3
        let case = make_query_case(vec!["gold-1", "gold-2", "gold-3"]);
        let result = make_case_result(vec![
            ("gold-1", 1, 0.9),
            ("other-1", 2, 0.8),
            ("gold-2", 3, 0.7),
            ("other-2", 4, 0.6),
            ("other-3", 5, 0.5),
        ]);
        assert!((recall_at_k(&case, &result, 5) - 2.0 / 3.0).abs() < 1e-6);
        // Only 1 of 3 in top-1
        assert!((recall_at_k(&case, &result, 1) - 1.0 / 3.0).abs() < 1e-6);
        // Hit rate is still true for partial multi-gold
        assert!(hit_at_k(&case, &result, 5));
        assert_ne!(
            recall_at_k(&case, &result, 5),
            if hit_at_k(&case, &result, 5) {
                1.0
            } else {
                0.0
            }
        );
    }

    #[test]
    fn test_recall_at_k_all_gold_found() {
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9), ("gold-2", 2, 0.8)]);
        assert!((recall_at_k(&case, &result, 5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_recall_at_k_empty_gold() {
        let case = make_query_case(vec![]);
        let result = make_case_result(vec![("other-1", 1, 0.9)]);
        assert_eq!(recall_at_k(&case, &result, 5), 0.0);
        assert!(!hit_at_k(&case, &result, 5));
    }

    #[test]
    fn test_recall_at_k_duplicate_gold_ids() {
        // Duplicate gold IDs collapse to unique set: still one gold item
        let case = make_query_case(vec!["gold-1", "gold-1", "gold-1"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9), ("other", 2, 0.7)]);
        assert!((recall_at_k(&case, &result, 5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_recall_at_k_no_match() {
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![("other-1", 1, 0.9), ("other-2", 2, 0.8)]);
        assert_eq!(recall_at_k(&case, &result, 10), 0.0);
    }

    #[test]
    fn test_reciprocal_rank_at_1() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9)]);
        assert_eq!(reciprocal_rank(&case, &result), 1.0);
    }

    #[test]
    fn test_reciprocal_rank_at_2() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("other", 1, 0.9), ("gold-1", 2, 0.7)]);
        assert_eq!(reciprocal_rank(&case, &result), 0.5);
    }

    #[test]
    fn test_reciprocal_rank_at_3() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![
            ("other-1", 1, 0.9),
            ("other-2", 2, 0.8),
            ("gold-1", 3, 0.7),
        ]);
        assert_eq!(reciprocal_rank(&case, &result), 1.0 / 3.0);
    }

    #[test]
    fn test_reciprocal_rank_no_match() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("other-1", 1, 0.9), ("other-2", 2, 0.8)]);
        assert_eq!(reciprocal_rank(&case, &result), 0.0);
    }

    #[test]
    fn test_ndcg_perfect_log2() {
        // All gold items at the top → NDCG = 1.0 under log2 discount
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9), ("gold-2", 2, 0.8)]);
        assert!((ndcg_at_k(&case, &result, 10) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ndcg_partial_log2() {
        // One gold at position 1 (i=0), one at position 5 (i=4)
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![
            ("gold-1", 1, 0.9),
            ("other-1", 2, 0.8),
            ("other-2", 3, 0.7),
            ("other-3", 4, 0.6),
            ("gold-2", 5, 0.5),
        ]);
        // DCG = 1/log2(2) + 1/log2(6) = 1.0 + 1/log2(6)
        // IDCG = 1/log2(2) + 1/log2(3) = 1.0 + 1/log2(3)
        let dcg = log_discount(0) + log_discount(4);
        let idcg = log_discount(0) + log_discount(1);
        let expected = dcg / idcg;
        assert!((ndcg_at_k(&case, &result, 10) - expected).abs() < 1e-5);
        // Partial ranking must be strictly less than perfect
        assert!(expected < 1.0);
    }

    #[test]
    fn test_ndcg_no_match() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("other-1", 1, 0.9), ("other-2", 2, 0.8)]);
        assert_eq!(ndcg_at_k(&case, &result, 10), 0.0);
    }

    #[test]
    fn test_ndcg_single_gold_at_top() {
        let case = make_query_case(vec!["gold-1"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9)]);
        assert!((ndcg_at_k(&case, &result, 10) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ndcg_empty_gold() {
        let case = make_query_case(vec![]);
        let result = make_case_result(vec![("other-1", 1, 0.9)]);
        assert_eq!(ndcg_at_k(&case, &result, 10), 0.0);
    }

    #[test]
    fn test_log_discount_formula() {
        // rank 1 → log2(2) = 1 → discount 1.0
        assert!((log_discount(0) - 1.0).abs() < 1e-6);
        // rank 3 → log2(4) = 2 → discount 0.5
        assert!((log_discount(2) - 0.5).abs() < 1e-6);
    }
}
