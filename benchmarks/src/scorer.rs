use crate::types::{CaseResult, QueryCase};

pub fn hit_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> bool {
    result
        .retrieved
        .iter()
        .take(k)
        .any(|r| case.gold_evidence_ids.iter().any(|id| id == &r.memory_id))
}

pub fn reciprocal_rank(case: &QueryCase, result: &CaseResult) -> f32 {
    for item in &result.retrieved {
        if case.gold_evidence_ids.iter().any(|id| id == &item.memory_id) {
            return 1.0 / item.rank as f32;
        }
    }
    0.0
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
            predicted_answer: None,
            exact_match: None,
            abstained: false,
            latency_ms: 0,
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
}
