use crate::types::{BenchmarkMetadata, CaseResult, SummaryMetrics};
use anyhow::Result;
use std::{fs, path::Path};

pub fn write_summary(path: &Path, summary: &SummaryMetrics) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(summary)?)?;
    Ok(())
}

pub fn write_results_jsonl(path: &Path, results: &[CaseResult]) -> Result<()> {
    let mut out = String::new();
    for r in results {
        out.push_str(&serde_json::to_string(r)?);
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

pub fn write_markdown(
    path: &Path,
    summary: &SummaryMetrics,
    metadata: &BenchmarkMetadata,
    summary_path: &Path,
    results_path: &Path,
) -> Result<()> {
    let commit_line = metadata.commit_sha.as_deref().unwrap_or("(not available)");
    let reader_mode = if metadata.reader_mode_enabled {
        "enabled"
    } else {
        "disabled"
    };

    let md = format!(
        "# Benchmark Report\n\n## Context\n\
- Dataset: `{}` (version {}, seed {}, sessions {})\n\
- Mode: {} (reader mode: {})\n\
- Retrieval top-k: {} | Abstain threshold: {:.2}\n\
- Commit: {}\n\
## Outputs\n\
- summary: `{}`\n\
- results: `{}`\n\
## Metrics\n\
- Cases: {}\n\
- Recall@1: {:.4}\n\
- Recall@5: {:.4}\n\
- Recall@10: {:.4}\n\
- MRR: {:.4}\n\
- NDCG@10: {:.4}\n\
- Abstain precision: {:.4}\n\
- Abstain recall: {:.4}\n\
- Ingest ms: {}\n\
- p50 latency ms: {}\n\
- p95 latency ms: {}\n\
- p99 latency ms: {}\n\
- Storage bytes: {}\n\
- Peak memory bytes: {}\n\
- Prompt tokens: {}\n\
- Completion tokens: {}\n",
        metadata.dataset_dir,
        metadata.dataset_version,
        metadata.dataset_seed,
        metadata.dataset_session_count,
        metadata.mode,
        reader_mode,
        metadata.top_k,
        metadata.abstain_threshold,
        commit_line,
        summary_path.display(),
        results_path.display(),
        summary.cases,
        summary.recall_at_1,
        summary.recall_at_5,
        summary.recall_at_10,
        summary.mrr,
        summary.ndcg_at_10,
        summary.abstain_precision,
        summary.abstain_recall,
        summary.ingest_ms,
        summary.p50_latency_ms,
        summary.p95_latency_ms,
        summary.p99_latency_ms,
        summary.storage_bytes,
        summary.peak_memory_bytes,
        summary.prompt_tokens,
        summary.completion_tokens
    );

    fs::write(path, md)?;
    Ok(())
}
