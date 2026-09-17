//! Local persistence contention evidence: sequential throughput, read path,
//! storage bytes, and a concurrent-write probe reported both raw (no retry)
//! and with a caller-side bounded retry policy.

use std::sync::Arc;
use std::time::Instant;

use serde_json::{Value, json};

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;

use super::util::{Corpus, file_bytes, summarize_us};

const NS: &str = "_default";
const BATCH: usize = 500;
/// Bounded-retry policy used for the second concurrency variant.
const RETRY_LIMIT: u32 = 5;
const RETRY_BACKOFF_MS: u64 = 2;

/// Parameters for the persistence probe.
pub struct PersistenceParams {
    /// Concept counts to save sequentially, ascending.
    pub scales: Vec<usize>,
    /// Concurrent tasks in the contention probe.
    pub tasks: usize,
    /// Round-trips per task in the contention probe.
    pub ops_per_task: usize,
}

pub async fn run(params: &PersistenceParams) -> Value {
    let mut scales = Vec::new();
    for &scale in &params.scales {
        scales.push(scale_point(scale).await);
    }

    json!({
        "kind": "persistence_scale",
        "batch_size": BATCH,
        "retry_policy": {
            "library_side": "none",
            "probe_side": { "limit": RETRY_LIMIT, "backoff_ms": RETRY_BACKOFF_MS },
        },
        "scales": scales,
        "contention": contention(params).await,
    })
}

/// Sequential save/read path at one concept count.
async fn scale_point(scale: usize) -> Value {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let db = dir.path().join("evidence.db");
    let path = db.to_str().expect("utf8 path").to_string();
    let persistence = Persistence::new_local(&path).await.expect("open db");

    let mut batch_latencies = Vec::new();
    let write_start = Instant::now();
    let mut batch: Vec<Concept> = Vec::with_capacity(BATCH);
    for concept in Corpus::stream_concepts(42, scale, 64, 512) {
        batch.push(concept);
        if batch.len() == BATCH {
            let start = Instant::now();
            persistence
                .save_concepts(NS, &batch)
                .await
                .expect("save batch");
            batch_latencies.push(start.elapsed().as_micros() as u64);
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let start = Instant::now();
        persistence
            .save_concepts(NS, &batch)
            .await
            .expect("save batch");
        batch_latencies.push(start.elapsed().as_micros() as u64);
    }
    let write_secs = write_start.elapsed().as_secs_f64();

    let read_start = Instant::now();
    let loaded = persistence
        .load_all_concepts::<chaotic_semantic_memory::HVec10240>(NS)
        .await
        .expect("load all");
    let read_ms = read_start.elapsed().as_secs_f64() * 1000.0;

    let pre_checkpoint = super::util::sqlite_bytes(&db);
    persistence.checkpoint().await.expect("checkpoint");
    let post_checkpoint = super::util::sqlite_bytes(&db);
    let reported_bytes = persistence.size().await.unwrap_or(0);

    json!({
        "concepts": scale,
        "loaded_concepts": loaded.len(),
        "write_secs": write_secs,
        "write_concepts_per_sec": scale as f64 / write_secs,
        "batch_latency": summarize_us(batch_latencies),
        "read_ms": read_ms,
        "read_concepts_per_sec": scale as f64 / (read_ms / 1000.0),
        "bytes_before_checkpoint": pre_checkpoint,
        "bytes_after_checkpoint": post_checkpoint,
        "size_api_bytes": reported_bytes,
    })
}

/// Concurrent writers on one local database.
///
/// `raw` performs no retries and reports the library's failure modes; `bounded`
/// retries `database is locked` up to [`RETRY_LIMIT`] times with a fixed
/// backoff, the policy a caller must currently implement because the library
/// exposes none (ADR-0095 requires this to be bounded and reported).
async fn contention(params: &PersistenceParams) -> Value {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let db = dir.path().join("contention.db");
    let path = db.to_str().expect("utf8 path").to_string();

    let persistence = Arc::new(Persistence::new_local(&path).await.expect("open db"));
    let raw = run_variant(&persistence, params.tasks, params.ops_per_task, false).await;
    let bounded = run_variant(&persistence, params.tasks, params.ops_per_task, true).await;

    json!({
        "tasks": params.tasks,
        "ops_per_task": params.ops_per_task,
        "raw": raw,
        "bounded_retry": bounded,
        "db_bytes": super::util::sqlite_bytes(&db),
        "wal_present": file_bytes(&std::path::PathBuf::from(format!("{}-wal", db.display()))) > 0,
    })
}

async fn run_variant(
    persistence: &Arc<Persistence>,
    tasks: usize,
    ops_per_task: usize,
    retry: bool,
) -> Value {
    let start = Instant::now();
    let mut handles = Vec::with_capacity(tasks);

    for task in 0..tasks {
        let store = Arc::clone(persistence);
        handles.push(tokio::spawn(async move {
            let mut latencies = Vec::with_capacity(ops_per_task);
            let mut retries = 0u64;
            let mut errors = 0u64;

            for op in 0..ops_per_task {
                let concept = Concept {
                    id: format!("t{task}-op{op}"),
                    vector: chaotic_semantic_memory::HVec10240::zero(),
                    metadata: Default::default(),
                    created_at: 1,
                    modified_at: 1,
                    expires_at: None,
                    canonical_concept_ids: Vec::new(),
                };
                let op_start = Instant::now();
                let mut attempts = 0;
                loop {
                    match store.save_concept(NS, &concept).await {
                        Ok(()) => break,
                        Err(e) if retry && attempts < RETRY_LIMIT && is_locked(&e) => {
                            attempts += 1;
                            retries += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(RETRY_BACKOFF_MS))
                                .await;
                        }
                        Err(_) => {
                            errors += 1;
                            break;
                        }
                    }
                }
                latencies.push(op_start.elapsed().as_micros() as u64);
            }
            (latencies, retries, errors)
        }));
    }

    let mut latencies = Vec::new();
    let mut retries = 0u64;
    let mut errors = 0u64;
    for handle in handles {
        let (mut l, r, e) = handle.await.expect("task join");
        latencies.append(&mut l);
        retries += r;
        errors += e;
    }

    let secs = start.elapsed().as_secs_f64();
    let ops = (tasks * ops_per_task) as f64;
    json!({
        "ops": tasks * ops_per_task,
        "secs": secs,
        "ops_per_sec": ops / secs,
        "latency": summarize_us(latencies),
        "retries": retries,
        "retry_rate": retries as f64 / ops,
        "errors": errors,
        "error_rate": errors as f64 / ops,
    })
}

fn is_locked(error: &impl std::fmt::Debug) -> bool {
    let text = format!("{error:?}").to_lowercase();
    text.contains("locked") || text.contains("busy")
}
