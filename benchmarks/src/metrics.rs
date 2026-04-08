use crate::types::{CaseResult, SummaryMetrics, TaskType};

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
    let p50 = latencies[count / 2];
    let p95 = latencies[(count as f32 * 0.95) as usize];

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
        exact_match,
        abstain_precision,
        abstain_recall,
        ingest_ms,
        p50_latency_ms: p50,
        p95_latency_ms: p95,
        storage_bytes,
        peak_memory_bytes: peak_memory,
        prompt_tokens: results.iter().map(|r| r.prompt_tokens as u64).sum(),
        completion_tokens: results.iter().map(|r| r.completion_tokens as u64).sum(),
    }
}
