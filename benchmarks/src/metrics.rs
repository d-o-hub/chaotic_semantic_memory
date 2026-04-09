use crate::types::{CaseResult, SummaryMetrics, TaskType};

/// Aggregates case results into summary metrics.
///
/// # Arguments
/// * `results` - List of individual query case results
/// * `ingest_ms` - Total time to ingest all memories
/// * `peak_memory` - Peak memory usage during benchmark
/// * `storage_bytes` - Estimated storage size of all memories
pub fn aggregate(
    results: &[CaseResult],
    ingest_ms: u128,
    peak_memory: u64,
    storage_bytes: u64,
) -> SummaryMetrics {
    let count = results.len();
    if count == 0 {
        return SummaryMetrics {
            ingest_ms,
            peak_memory_bytes: peak_memory,
            storage_bytes,
            ..Default::default()
        };
    }

    let recall_at_1 = results.iter().filter(|r| r.recall_at_1).count() as f32 / count as f32;
    let recall_at_5 = results.iter().filter(|r| r.recall_at_5).count() as f32 / count as f32;
    let recall_at_10 = results.iter().filter(|r| r.recall_at_10).count() as f32 / count as f32;
    let mrr = results.iter().map(|r| r.reciprocal_rank).sum::<f32>() / count as f32;
    let ndcg_at_10 = results.iter().map(|r| r.ndcg_at_10).sum::<f32>() / count as f32;

    let abstain_cases: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.task_type, TaskType::Abstain))
        .collect();
    let abstain_count = abstain_cases.len();

    let (abstain_precision, abstain_recall) = if abstain_count > 0 {
        let true_positives = abstain_cases.iter().filter(|r| r.abstained).count() as f32;
        let false_positives = results
            .iter()
            .filter(|r| !matches!(r.task_type, TaskType::Abstain) && r.abstained)
            .count() as f32;

        let precision = true_positives / (true_positives + false_positives).max(1.0);
        let recall = true_positives / abstain_count as f32;
        (precision, recall)
    } else {
        (0.0, 0.0)
    };

    let mut latencies: Vec<_> = results.iter().map(|r| r.latency_ms).collect();
    latencies.sort_unstable();
    // Use floor-based indexing for percentiles (industry standard)
    let p50 = latencies[(count - 1) / 2];  // True lower median
    let p95_idx = ((count - 1) as f64 * 0.95) as usize;  // Floor via truncation
    let p95 = latencies[p95_idx];
    let p99_idx = ((count - 1) as f64 * 0.99) as usize;  // Floor via truncation
    let p99 = latencies[p99_idx];

    let exact_matches: Vec<_> = results.iter().filter_map(|r| r.exact_match).collect();
    let exact_match = if !exact_matches.is_empty() {
        Some(exact_matches.iter().filter(|&&m| m).count() as f32 / exact_matches.len() as f32)
    } else {
        None
    };

    SummaryMetrics {
        cases: count,
        recall_at_1,
        recall_at_5,
        recall_at_10,
        mrr,
        ndcg_at_10,
        exact_match,
        abstain_precision,
        abstain_recall,
        ingest_ms,
        p50_latency_ms: p50,
        p95_latency_ms: p95,
        p99_latency_ms: p99,
        storage_bytes,
        peak_memory_bytes: peak_memory,
        prompt_tokens: results.iter().map(|r| r.prompt_tokens as u64).sum(),
        completion_tokens: results.iter().map(|r| r.completion_tokens as u64).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RetrievedItem;

    fn make_result(
        query_id: &str,
        task_type: TaskType,
        recall_at_1: bool,
        recall_at_5: bool,
        latency_ms: u128,
        abstained: bool,
    ) -> CaseResult {
        CaseResult {
            query_id: query_id.into(),
            task_type,
            retrieved: vec![RetrievedItem {
                memory_id: "test".into(),
                rank: 1,
                score: 0.5,
            }],
            recall_at_1,
            recall_at_5,
            recall_at_10: recall_at_5,
            reciprocal_rank: if recall_at_1 { 1.0 } else { 0.0 },
            ndcg_at_10: if recall_at_1 { 1.0 } else { 0.0 },
            predicted_answer: None,
            exact_match: None,
            abstained,
            latency_ms,
            prompt_tokens: 0,
            completion_tokens: 0,
        }
    }

    #[test]
    fn aggregate_empty_results() {
        let summary = aggregate(&[], 100, 1024, 512);
        assert_eq!(summary.cases, 0);
        assert_eq!(summary.ingest_ms, 100);
        assert_eq!(summary.peak_memory_bytes, 1024);
        assert_eq!(summary.storage_bytes, 512);
    }

    #[test]
    fn aggregate_single_result() {
        let results = vec![make_result("q1", TaskType::Recall, true, true, 10, false)];
        let summary = aggregate(&results, 100, 1024, 512);
        assert_eq!(summary.cases, 1);
        assert_eq!(summary.recall_at_1, 1.0);
        assert_eq!(summary.recall_at_5, 1.0);
        assert_eq!(summary.mrr, 1.0);
        // Single element: p50, p95, p99 should all be the same
        assert_eq!(summary.p50_latency_ms, 10);
        assert_eq!(summary.p95_latency_ms, 10);
        assert_eq!(summary.p99_latency_ms, 10);
    }

    #[test]
    fn aggregate_recall_mixed() {
        let results = vec![
            make_result("q1", TaskType::Recall, true, true, 10, false),
            make_result("q2", TaskType::Recall, false, false, 20, false),
        ];
        let summary = aggregate(&results, 100, 1024, 512);
        assert_eq!(summary.cases, 2);
        assert_eq!(summary.recall_at_1, 0.5);
        assert_eq!(summary.recall_at_5, 0.5);
        assert_eq!(summary.mrr, 0.5);
    }

    #[test]
    fn aggregate_abstention_metrics() {
        let results = vec![
            make_result("q1", TaskType::Abstain, false, false, 10, true),  // true positive
            make_result("q2", TaskType::Abstain, false, false, 10, false), // false negative
            make_result("q3", TaskType::Recall, false, false, 10, true),   // false positive
            make_result("q4", TaskType::Recall, false, false, 10, false),  // true negative
        ];
        let summary = aggregate(&results, 100, 1024, 512);
        // 1 TP, 1 FP -> precision = 1/2 = 0.5
        // 2 abstain cases, 1 abstained -> recall = 1/2 = 0.5
        assert_eq!(summary.abstain_precision, 0.5);
        assert_eq!(summary.abstain_recall, 0.5);
    }

    #[test]
    fn aggregate_latency_percentiles() {
        let results: Vec<_> = (1..=10)
            .map(|i| make_result(&format!("q{}", i), TaskType::Recall, false, false, i, false))
            .collect();
        let summary = aggregate(&results, 100, 1024, 512);
        // Sorted latencies: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        // p50: (10-1) / 2 = 4 (floor) -> index 4 = 5 (corrected)
        // p95: (10-1) * 0.95 = 8.55 -> floor = 8 -> index 8 = 9 (corrected)
        // p99: (10-1) * 0.99 = 8.91 -> floor = 8 -> index 8 = 9
        assert_eq!(summary.p50_latency_ms, 5);
        assert_eq!(summary.p95_latency_ms, 9);
        assert_eq!(summary.p99_latency_ms, 9);
    }
}
