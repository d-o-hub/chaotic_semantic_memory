use serde::{Deserialize, Serialize};

/// Type of benchmark task for a query case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Simple memory recall task.
    Recall,
    /// Update/correction task where newer information supersedes old.
    Update,
    /// Temporal reasoning task requiring time-based context.
    Temporal,
    /// Abstention task where the system should decline to answer.
    Abstain,
    /// Association task requiring cross-session linking.
    Association,
    /// Multi-session task requiring aggregation across sessions.
    MultiSession,
}

/// A single turn in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    /// Timestamp of the turn (ISO 8601 format).
    pub ts: String,
    /// Speaker identifier (e.g., "user", "assistant").
    pub speaker: String,
    /// Text content of the turn.
    pub text: String,
    /// Optional memory ID assigned to this turn.
    pub memory_id: Option<String>,
}

/// A conversation session containing multiple turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for the session.
    pub session_id: String,
    /// List of turns in chronological order.
    pub turns: Vec<SessionTurn>,
}

/// A single query case for benchmark evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCase {
    /// Unique identifier for this query.
    pub query_id: String,
    /// ID of the session this query relates to.
    pub session_id: String,
    /// Type of task this query represents.
    pub task_type: TaskType,
    /// The query text.
    pub query: String,
    /// IDs of memory items that should be retrieved (gold evidence).
    pub gold_evidence_ids: Vec<String>,
    /// Expected answer for answer-quality evaluation (optional).
    pub expected_answer: Option<String>,
    /// Whether the system should abstain from answering.
    pub should_abstain: bool,
}

/// A single retrieved item from the memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedItem {
    /// Memory ID of the retrieved item.
    pub memory_id: String,
    /// Rank position in the result list (1-indexed).
    pub rank: usize,
    /// Similarity score from the memory system.
    pub score: f32,
}

/// Result for a single query case evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    /// Query ID this result corresponds to.
    pub query_id: String,
    /// Task type of the query.
    pub task_type: TaskType,
    /// List of retrieved items.
    pub retrieved: Vec<RetrievedItem>,
    /// Whether gold evidence was found at rank 1.
    pub recall_at_1: bool,
    /// Whether gold evidence was found in top 5.
    pub recall_at_5: bool,
    /// Whether gold evidence was found in top 10.
    pub recall_at_10: bool,
    /// Reciprocal rank of first gold evidence match.
    pub reciprocal_rank: f32,
    /// NDCG@10 for multi-gold evidence cases.
    pub ndcg_at_10: f32,
    /// Predicted answer (in reader-lite mode).
    pub predicted_answer: Option<String>,
    /// Whether predicted answer matches expected (exact match).
    pub exact_match: Option<bool>,
    /// Whether the system abstained from answering.
    pub abstained: bool,
    /// Query latency in milliseconds.
    pub latency_ms: u128,
    /// Query latency in microseconds (precise for sub-ms measurements).
    #[serde(default)]
    pub latency_us: u128,
    /// Number of prompt tokens used (reader mode).
    pub prompt_tokens: u32,
    /// Number of completion tokens used (reader mode).
    pub completion_tokens: u32,
}

/// Aggregated metrics across all query cases.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummaryMetrics {
    /// Total number of query cases evaluated.
    pub cases: usize,
    /// Fraction of cases with gold at rank 1.
    pub recall_at_1: f32,
    /// Fraction of cases with gold in top 5.
    pub recall_at_5: f32,
    /// Fraction of cases with gold in top 10.
    pub recall_at_10: f32,
    /// Mean reciprocal rank across all cases.
    pub mrr: f32,
    /// Mean NDCG@10 across all cases.
    #[serde(default)]
    pub ndcg_at_10: f32,
    /// Exact match accuracy (reader mode only).
    pub exact_match: Option<f32>,
    /// Precision of abstention decisions.
    pub abstain_precision: f32,
    /// Recall of abstention decisions.
    pub abstain_recall: f32,
    /// Total ingestion time in milliseconds.
    pub ingest_ms: u128,
    /// Median (p50) query latency in milliseconds.
    pub p50_latency_ms: u128,
    /// Median (p50) query latency in microseconds (precise for sub-ms).
    #[serde(default)]
    pub p50_latency_us: u128,
    /// 95th percentile query latency in milliseconds.
    pub p95_latency_ms: u128,
    /// 99th percentile query latency in milliseconds.
    #[serde(default)]
    pub p99_latency_ms: u128,
    /// Estimated storage size in bytes.
    pub storage_bytes: u64,
    /// Peak memory usage during benchmark in bytes.
    pub peak_memory_bytes: u64,
    /// Total prompt tokens used (reader mode).
    pub prompt_tokens: u64,
    /// Total completion tokens used (reader mode).
    pub completion_tokens: u64,
}

/// Dataset manifest metadata loaded from `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub version: String,
    pub seed: u64,
    pub session_count: usize,
}

/// High-level benchmark metadata recorded alongside reports.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkMetadata {
    pub dataset_dir: String,
    pub dataset_version: String,
    pub dataset_seed: u64,
    pub dataset_session_count: usize,
    pub mode: String,
    pub reader_mode_enabled: bool,
    pub top_k: usize,
    pub abstain_threshold: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}
