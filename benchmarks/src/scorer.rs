use crate::types::{CaseResult, QueryCase};
use std::collections::HashSet;

pub fn hit_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> bool {
    // Use HashSet for O(1) lookup of gold evidence IDs
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    result
        .retrieved
        .iter()
        .take(k)
        .any(|r| gold.contains(r.memory_id.as_str()))
}

pub fn reciprocal_rank(case: &QueryCase, result: &CaseResult) -> f32 {
    // Use HashSet for O(1) lookup of gold evidence IDs
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    for item in &result.retrieved {
        if gold.contains(item.memory_id.as_str()) {
            return 1.0 / item.rank as f32;
        }
    }
    0.0
}

/// Compute Normalized Discounted Cumulative Gain at k.
/// Uses logarithmic discount: relevance / 2^(position) where position is 1-indexed.
/// For binary relevance (gold/not-gold), this rewards systems that return relevant
/// items higher in the result list.
pub fn ndcg_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> f32 {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();

    // Compute DCG: sum of relevance / 2^(position) for each relevant item found
    let dcg: f32 = result
        .retrieved
        .iter()
        .take(k)
        .enumerate()
        .filter(|(_, r)| gold.contains(r.memory_id.as_str()))
        .map(|(i, _)| 1.0 / 2.0_f32.powi(i as i32))
        .sum();

    // Compute ideal DCG: what we'd get if all gold items were at the top
    let ideal_dcg: f32 = case
        .gold_evidence_ids
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, _)| 1.0 / 2.0_f32.powi(i as i32))
        .sum();

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
            task_type: TaskType::Recall,
            retrieved: retrieved
                .iter()
                .map(|(id, rank, score)| RetrievedItem {
                    memory_id: id.to_string(),
                    rank: *rank,
                    score: *score,
                })
                .collect(),
            recall_at_1: false,
            recall_at_5: false,
            recall_at_10: false,
            reciprocal_rank: 0.0,
            ndcg_at_10: 0.0,
            predicted_answer: None,
            exact_match: None,
            abstained: false,
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
    fn test_ndcg_perfect() {
        // All gold items at the top
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![("gold-1", 1, 0.9), ("gold-2", 2, 0.8)]);
        assert!((ndcg_at_k(&case, &result, 10) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ndcg_partial() {
        // One gold at position 1, one at position 5
        let case = make_query_case(vec!["gold-1", "gold-2"]);
        let result = make_case_result(vec![
            ("gold-1", 1, 0.9),
            ("other-1", 2, 0.8),
            ("other-2", 3, 0.7),
            ("other-3", 4, 0.6),
            ("gold-2", 5, 0.5),
        ]);
        // DCG = 1/2^0 + 1/2^4 = 1.0625
        // IDCG = 1/2^0 + 1/2^1 = 1.5
        // NDCG = 1.0625 / 1.5 = 0.70833...
        let expected = (1.0 + 1.0 / 16.0) / (1.0 + 0.5);
        assert!((ndcg_at_k(&case, &result, 10) - expected).abs() < 1e-5);
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
}
