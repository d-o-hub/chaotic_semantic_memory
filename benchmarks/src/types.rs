use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Recall,
    Update,
    Temporal,
    Abstain,
    Association,
    MultiSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub ts: String,
    pub speaker: String,
    pub text: String,
    pub memory_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub turns: Vec<SessionTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCase {
    pub query_id: String,
    pub session_id: String,
    pub task_type: TaskType,
    pub query: String,
    pub gold_evidence_ids: Vec<String>,
    pub expected_answer: Option<String>,
    pub should_abstain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedItem {
    pub memory_id: String,
    pub rank: usize,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub query_id: String,
    pub task_type: TaskType,
    pub retrieved: Vec<RetrievedItem>,
    pub recall_at_1: bool,
    pub recall_at_5: bool,
    pub recall_at_10: bool,
    pub reciprocal_rank: f32,
    pub predicted_answer: Option<String>,
    pub exact_match: Option<bool>,
    pub abstained: bool,
    pub latency_ms: u128,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummaryMetrics {
    pub cases: usize,
    pub recall_at_1: f32,
    pub recall_at_5: f32,
    pub recall_at_10: f32,
    pub mrr: f32,
    pub exact_match: Option<f32>,
    pub abstain_precision: f32,
    pub abstain_recall: f32,
    pub ingest_ms: u128,
    pub p50_latency_ms: u128,
    pub p95_latency_ms: u128,
    pub storage_bytes: u64,
    pub peak_memory_bytes: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}
