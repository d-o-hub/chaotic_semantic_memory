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

    // Mean multi-label recall / MRR / NDCG over cases that should answer (not abstain gold)
    let (recall_at_1, recall_at_5, recall_at_10, mrr, ndcg_at_10) = {
        let gold_cases: Vec<_> = results.iter().filter(|r| !r.should_abstain).collect();
        let gold_count = gold_cases.len();

        if gold_count > 0 {
            (
                gold_cases.iter().map(|r| r.recall_at_1).sum::<f32>() / gold_count as f32,
                gold_cases.iter().map(|r| r.recall_at_5).sum::<f32>() / gold_count as f32,
                gold_cases.iter().map(|r| r.recall_at_10).sum::<f32>() / gold_count as f32,
                gold_cases.iter().map(|r| r.reciprocal_rank).sum::<f32>() / gold_count as f32,
                gold_cases.iter().map(|r| r.ndcg_at_10).sum::<f32>() / gold_count as f32,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        }
    };

    // Abstention confusion matrix uses QueryCase.should_abstain as gold label,
    // not task-type inference alone (ADR-0095).
    let should_abstain_count = results.iter().filter(|r| r.should_abstain).count();
    let true_positives = results
        .iter()
        .filter(|r| r.should_abstain && r.abstained)
        .count() as f32;
    let false_positives = results
        .iter()
        .filter(|r| !r.should_abstain && r.abstained)
        .count() as f32;

    let (abstain_precision, abstain_recall) = if should_abstain_count > 0 || false_positives > 0.0 {
        let precision = if true_positives + false_positives > 0.0 {
            true_positives / (true_positives + false_positives)
        } else {
            0.0
        };
        let recall = if should_abstain_count > 0 {
            true_positives / should_abstain_count as f32
        } else {
            0.0
        };
        (precision, recall)
    } else {
        (0.0, 0.0)
    };

    let association_cases: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.task_type, TaskType::Association))
        .collect();
    let association_success_rate = if !association_cases.is_empty() {
        association_cases.iter().map(|r| r.recall_at_5).sum::<f32>()
            / association_cases.len() as f32
    } else {
        0.0
    };

    let multisession_cases: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.task_type, TaskType::MultiSession))
        .collect();
    let multisession_recall = if !multisession_cases.is_empty() {
        multisession_cases
            .iter()
            .map(|r| r.recall_at_5)
            .sum::<f32>()
            / multisession_cases.len() as f32
    } else {
        0.0
    };

    let isolation_cases: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.task_type, TaskType::Isolation))
        .collect();
    let session_isolation = if !isolation_cases.is_empty() {
        // For isolation, success means we correctly returned nothing (or below threshold)
        // because the query was for data in a different session.
        isolation_cases
            .iter()
            .filter(|r| r.retrieved.is_empty() || r.abstained)
            .count() as f32
            / isolation_cases.len() as f32
    } else {
        0.0
    };

    let mut latencies: Vec<_> = results.iter().map(|r| r.latency_ms).collect();
    latencies.sort_unstable();
    // Use floor-based indexing for percentiles (industry standard)
    let p50 = latencies[(count - 1) / 2]; // True lower median
    let p95_idx = ((count - 1) as f64 * 0.95) as usize; // Floor via truncation
    let p95 = latencies[p95_idx];
    let p99_idx = ((count - 1) as f64 * 0.99) as usize; // Floor via truncation
    let p99 = latencies[p99_idx];

    // Compute microsecond latencies for sub-ms precision
    let mut latencies_us: Vec<_> = results.iter().map(|r| r.latency_us).collect();
    latencies_us.sort_unstable();
    let p50_us = latencies_us[(count - 1) / 2];

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
        association_success_rate,
        multisession_recall,
        session_isolation,
        ingest_ms,
        p50_latency_ms: p50,
        p50_latency_us: p50_us,
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
        recall_at_1: f32,
        recall_at_5: f32,
        latency_ms: u128,
        abstained: bool,
        should_abstain: bool,
    ) -> CaseResult {
        CaseResult {
            query_id: query_id.into(),
            session_id: "test-session".into(),
            task_type,
            retrieved: vec![RetrievedItem {
                memory_id: "test".into(),
                rank: 1,
                score: 0.5,
            }],
            recall_at_1,
            recall_at_5,
            recall_at_10: recall_at_5,
            reciprocal_rank: if recall_at_1 > 0.0 { 1.0 } else { 0.0 },
            ndcg_at_10: if recall_at_1 > 0.0 { 1.0 } else { 0.0 },
            predicted_answer: None,
            exact_match: None,
            abstained,
            should_abstain,
            latency_ms,
            latency_us: latency_ms * 1000, // Simulate us from ms for tests
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
        let results = vec![make_result(
            "q1",
            TaskType::Recall,
            1.0,
            1.0,
            10,
            false,
            false,
        )];
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
            make_result("q1", TaskType::Recall, 1.0, 1.0, 10, false, false),
            make_result("q2", TaskType::Recall, 0.0, 0.0, 20, false, false),
        ];
        let summary = aggregate(&results, 100, 1024, 512);
        assert_eq!(summary.cases, 2);
        assert_eq!(summary.recall_at_1, 0.5);
        assert_eq!(summary.recall_at_5, 0.5);
        assert_eq!(summary.mrr, 0.5);
    }

    #[test]
    fn aggregate_multi_label_mean_recall() {
        // Fractional multi-label recall averages correctly
        let results = vec![
            make_result("q1", TaskType::Recall, 1.0, 2.0 / 3.0, 10, false, false),
            make_result("q2", TaskType::Recall, 0.5, 0.5, 10, false, false),
        ];
        let summary = aggregate(&results, 100, 1024, 512);
        assert!((summary.recall_at_1 - 0.75).abs() < 1e-6);
        assert!((summary.recall_at_5 - (2.0 / 3.0 + 0.5) / 2.0).abs() < 1e-6);
    }

    #[test]
    fn aggregate_abstention_metrics() {
        let results = vec![
            make_result("q1", TaskType::Abstain, 0.0, 0.0, 10, true, true), // TP
            make_result("q2", TaskType::Abstain, 0.0, 0.0, 10, false, true), // FN
            make_result("q3", TaskType::Recall, 0.0, 0.0, 10, true, false), // FP
            make_result("q4", TaskType::Recall, 0.0, 0.0, 10, false, false), // TN
        ];
        let summary = aggregate(&results, 100, 1024, 512);
        // 1 TP, 1 FP -> precision = 1/2 = 0.5
        // 2 should_abstain, 1 abstained -> recall = 1/2 = 0.5
        assert_eq!(summary.abstain_precision, 0.5);
        assert_eq!(summary.abstain_recall, 0.5);
    }

    #[test]
    fn aggregate_abstention_label_disagreement() {
        // should_abstain=true but task is NOT Abstain (e.g. Isolation / custom label)
        let results = vec![
            make_result("q1", TaskType::Isolation, 0.0, 0.0, 10, true, true), // TP via label
            make_result("q2", TaskType::Recall, 0.0, 0.0, 10, true, true),    // TP: label not task
            make_result("q3", TaskType::Abstain, 0.0, 0.0, 10, false, false), // TN: task Abstain but gold says no
            make_result("q4", TaskType::Recall, 0.0, 0.0, 10, true, false),   // FP
        ];
        let summary = aggregate(&results, 100, 1024, 512);
        // TP=2 (q1,q2), FP=1 (q4) → precision = 2/3
        // should_abstain=2 (q1,q2), TP=2 → recall = 1.0
        // q3 has task Abstain but should_abstain=false → not counted as gold positive
        assert!((summary.abstain_precision - 2.0 / 3.0).abs() < 1e-6);
        assert!((summary.abstain_recall - 1.0).abs() < 1e-6);
        // Gold-answer cases exclude should_abstain; only q3+q4 contribute to mean recall (both 0)
        assert_eq!(summary.recall_at_1, 0.0);
    }

    #[test]
    fn aggregate_latency_percentiles() {
        let results: Vec<_> = (1..=10)
            .map(|i| {
                make_result(
                    &format!("q{i}"),
                    TaskType::Recall,
                    0.0,
                    0.0,
                    i,
                    false,
                    false,
                )
            })
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
