use crate::{
    cli::{Cli, Mode},
    dataset,
    memory_adapter::MemoryAdapter,
    metrics,
    reader::Reader,
    report,
    scorer,
    types::{CaseResult, RetrievedItem},
};
use anyhow::Result;
use std::time::Instant;
use sysinfo::{System, Pid};

pub async fn run(cli: Cli) -> Result<()> {
    println!("Loading dataset from {}", cli.dataset_dir.display());
    let sessions = dataset::load_sessions(&cli.dataset_dir.join("sessions.jsonl"))?;
    let queries = dataset::load_queries(&cli.dataset_dir.join("queries.jsonl"))?;

    println!("Initializing memory system...");
    let adapter = MemoryAdapter::new_in_memory().await?;
    let reader = Reader::new();

    let mut sys = System::new_all();
    let pid = Pid::from_u32(std::process::id());

    sys.refresh_all();
    let mut peak_mem = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

    println!("Ingesting memories...");
    let start_ingest = Instant::now();
    for session in &sessions {
        for turn in &session.turns {
            if let Some(id) = &turn.memory_id {
                adapter.ingest_memory(id, &turn.text).await?;
            }
        }
        sys.refresh_all();
        peak_mem = peak_mem.max(sys.process(pid).map(|p| p.memory()).unwrap_or(0));
    }
    let ingest_ms = start_ingest.elapsed().as_millis();

    println!("Running queries...");
    let mut results = Vec::new();
    for query_case in queries {
        let start_query = Instant::now();
        let hits = adapter.query(&query_case.query, cli.top_k).await?;
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

        // Simple abstention logic for retrieval-only: if top score < threshold or empty
        result.abstained = result.retrieved.is_empty() || result.retrieved[0].score < 0.1;

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

        sys.refresh_all();
        peak_mem = peak_mem.max(sys.process(pid).map(|p| p.memory()).unwrap_or(0));
    }

    println!("Aggregating metrics...");
    let summary = metrics::aggregate(&results, ingest_ms, peak_mem, 0);

    println!("Writing reports to {}...", cli.out_dir.display());
    std::fs::create_dir_all(&cli.out_dir)?;
    report::write_summary(&cli.out_dir.join("summary.json"), &summary)?;
    report::write_results_jsonl(&cli.out_dir.join("results.jsonl"), &results)?;
    report::write_markdown(&cli.out_dir.join("report.md"), &summary)?;

    println!("Benchmark complete.");
    println!("Recall@1: {:.4}", summary.recall_at_1);
    println!("MRR: {:.4}", summary.mrr);
    println!("Peak memory: {} bytes", summary.peak_memory_bytes);

    Ok(())
}
