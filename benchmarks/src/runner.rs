use crate::{
    cli::{Cli, Mode},
    dataset,
    memory_adapter::MemoryAdapter,
    metrics,
    reader::Reader,
    report,
    scorer,
    types::{BenchmarkMetadata, CaseResult, RetrievedItem, TaskType},
};
use anyhow::Result;
use std::{process::Command, time::Instant};
use sysinfo::{System, Pid, ProcessesToUpdate};

pub async fn run(cli: Cli) -> Result<()> {
    println!("Loading dataset from {}", cli.dataset_dir.display());
    let sessions = dataset::load_sessions(&cli.dataset_dir.join("sessions.jsonl"))?;
    let queries = dataset::load_queries(&cli.dataset_dir.join("queries.jsonl"))?;
    let manifest = dataset::load_manifest(&cli.dataset_dir.join("manifest.json"))?;

    println!("Initializing memory system...");
    let adapter = MemoryAdapter::new_in_memory().await?;
    let reader = Reader::new();

    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());

    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    let mut peak_mem = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

    println!("Ingesting memories...");
    let start_ingest = Instant::now();
    for session in &sessions {
        for turn in &session.turns {
            if let Some(id) = &turn.memory_id {
                adapter.ingest_memory(id, &turn.text).await?;
            }
        }
    }
    // Sample memory only once after all ingest
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    peak_mem = peak_mem.max(sys.process(pid).map(|p| p.memory()).unwrap_or(0));
    let ingest_ms = start_ingest.elapsed().as_millis();

    println!("Running queries...");
    let mut results = Vec::new();
    for query_case in queries {
        let start_query = Instant::now();

        // Use session-scoped retrieval for session-specific queries
        let hits = if matches!(
            query_case.task_type,
            TaskType::Recall | TaskType::Update | TaskType::Temporal | TaskType::Abstain,
        ) {
            adapter.query_in_session(&query_case.query, &query_case.session_id, cli.top_k).await?
        } else {
            // For abstain and other queries, search globally
            adapter.query(&query_case.query, cli.top_k).await?
        };
        let latency_ms = start_query.elapsed().as_millis();

        let retrieved: Vec<_> = hits
            .into_iter()
            .enumerate()
            .map(|(i, (id, score))| RetrievedItem {
                memory_id: id,
                rank: i + 1,
                score,
            })
            .collect();

        let mut result = CaseResult {
            query_id: query_case.query_id.clone(),
            task_type: query_case.task_type.clone(),
            retrieved,
            recall_at_1: false,
            recall_at_5: false,
            recall_at_10: false,
            reciprocal_rank: 0.0,
            ndcg_at_10: 0.0,
            predicted_answer: None,
            exact_match: None,
            abstained: false,
            latency_ms,
            prompt_tokens: 0,
            completion_tokens: 0,
        };

        result.recall_at_1 = scorer::hit_at_k(&query_case, &result, 1);
        result.recall_at_5 = scorer::hit_at_k(&query_case, &result, 5);
        result.recall_at_10 = scorer::hit_at_k(&query_case, &result, 10);
        result.reciprocal_rank = scorer::reciprocal_rank(&query_case, &result);
        result.ndcg_at_10 = scorer::ndcg_at_k(&query_case, &result, 10);

        // Simple abstention logic for retrieval-only: if top score < threshold or empty
        result.abstained = result.retrieved.is_empty() || result.retrieved[0].score < cli.abstain_threshold;

        if matches!(cli.mode, Mode::ReaderLite) {
            let mut retrieved_texts = Vec::new();
            for item in &result.retrieved {
                if let Some(text) = adapter.get_text(&item.memory_id).await? {
                    retrieved_texts.push(text);
                }
            }

            let (prediction, p_tokens, c_tokens) =
                reader.predict(&query_case, &retrieved_texts).await?;
            result.predicted_answer = Some(prediction.clone());
            result.prompt_tokens = p_tokens;
            result.completion_tokens = c_tokens;
            result.exact_match =
                Some(reader.score_exact_match(&prediction, query_case.expected_answer.as_ref()));

            if prediction.contains("don't have enough information") {
                result.abstained = true;
            }
        }

        results.push(result);

        // Sample memory every 10 queries for accurate peak measurement
        if results.len() % 10 == 0 {
            sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
            peak_mem = peak_mem.max(sys.process(pid).map(|p| p.memory()).unwrap_or(0));
        }
    }

    // Sample memory once after all queries
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    peak_mem = peak_mem.max(sys.process(pid).map(|p| p.memory()).unwrap_or(0));

    let storage_bytes = adapter.storage_bytes().await?;

    println!("Aggregating metrics...");
    let summary = metrics::aggregate(&results, ingest_ms, peak_mem, storage_bytes);

    let metadata = BenchmarkMetadata {
        dataset_dir: cli.dataset_dir.display().to_string(),
        dataset_version: manifest.version,
        dataset_seed: manifest.seed,
        dataset_session_count: manifest.session_count,
        mode: cli.mode.to_string(),
        reader_mode_enabled: matches!(cli.mode, Mode::ReaderLite),
        top_k: cli.top_k,
        abstain_threshold: cli.abstain_threshold,
        commit_sha: resolve_commit_sha(),
    };

    println!("Writing reports to {}...", cli.out_dir.display());
    std::fs::create_dir_all(&cli.out_dir)?;
    let summary_path = cli.out_dir.join("summary.json");
    let results_path = cli.out_dir.join("results.jsonl");
    let report_path = cli.out_dir.join("report.md");
    report::write_summary(&summary_path, &summary)?;
    report::write_results_jsonl(&results_path, &results)?;
    report::write_markdown(&report_path, &summary, &metadata, &summary_path, &results_path)?;

    println!("Benchmark complete.");
    println!("Recall@1: {:.4}", summary.recall_at_1);
    println!("MRR: {:.4}", summary.mrr);
    println!("Peak memory: {} bytes", summary.peak_memory_bytes);

    Ok(())
}

fn resolve_commit_sha() -> Option<String> {
    const ENV_GITHUB_SHA: &str = "GITHUB_SHA";
    const ENV_PATH: &str = "PATH";

    if let Ok(sha) = std::env::var(ENV_GITHUB_SHA) {
        if !sha.trim().is_empty() {
            return Some(sha);
        }
    }

    // Filter PATH to exclude relative entries (CWE-426) to prevent path hijacking.
    // If PATH is unset or results in an empty string after filtering, we fallback
    // to letting the system attempt to find 'git' normally (standard behavior).
    let safe_path = std::env::var(ENV_PATH).ok().and_then(|p| {
        let joined = std::env::join_paths(
            std::env::split_paths(&p).filter(|p| p.is_absolute() && p.exists()),
        )
        .unwrap_or_default();
        if joined.to_string_lossy().is_empty() {
            None
        } else {
            Some(joined)
        }
    });

    let mut cmd = Command::new("git");
    if let Some(path) = safe_path {
        cmd.env(ENV_PATH, path);
    }
    let output = cmd
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
