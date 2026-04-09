# Benchmark Optimization Implementation Plan

## GOAP Summary

**Goal**: Optimize chaotic_semantic_memory benchmark suite for production-ready retrieval evaluation

**Total Actions**: 16 optimization actions across 6 files
**Estimated LOC Impact**: ~150 lines
**Estimated Cost**: 51 action units (effort weighting)

---

## Phase 1: Low-Cost Correctness Fixes (Priority: CRITICAL)

### 1.1 Fix Percentile Indexing (metrics.rs:53-55)

**Current**:
```rust
let p50 = latencies[count / 2];  // Biased for even n
let p95_idx = ((count - 1) as f64 * 0.95).round() as usize;  // Can overshoot
```

**Fix**:
```rust
let p50 = latencies[(count - 1) / 2];  // True lower median
let p95_idx = ((count - 1) as f64 * 0.95) as usize;  // Floor via truncation
let p99_idx = ((count - 1) as f64 * 0.99) as usize;  // Add p99
let p99 = latencies[p99_idx];
```

### 1.2 Add Defensive Sort Guard (scorer.rs:4-8)

**Current**: Assumes results pre-sorted by rank
**Fix**:
```rust
pub fn hit_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> bool {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    result.retrieved.iter()
        .take(k)
        .any(|r| gold.contains(r.memory_id.as_str()))
}
```

Note: Defensive sort not needed since `enumerate()` in runner.rs assigns correct ranks. HashSet optimization still valuable.

### 1.3 Fix sysinfo Sampling (runner.rs:27,40,117)

**Current**: `sys.refresh_all()` - expensive, reads all processes
**Fix**:
```rust
let pid = Pid::from_u32(std::process::id());
sys.refresh_process(pid);  // Only refresh our process
peak_mem = peak_mem.max(sys.process(pid).map(|p| p.memory()).unwrap_or(0));
```

Do this inside query loop every 10 queries to capture peak accurately.

---

## Phase 2: Metrics Enhancements (Priority: HIGH)

### 2.1 Add p99 Latency (types.rs, metrics.rs)

Add to SummaryMetrics struct:
```rust
pub p99_latency_ms: u128,
```

Update aggregate() to compute and include.

### 2.2 Per-Task Breakdown (types.rs, metrics.rs)

Add to SummaryMetrics:
```rust
pub per_task_metrics: HashMap<TaskType, TaskMetrics>,
```

Where TaskMetrics contains partial recall/MRR/latency stats.

### 2.3 Single-Pass Aggregation (metrics.rs:26-79)

**Current**: 6+ iterator passes
**Fix** - single fold:
```rust
let (r1, r5, r10, mrr_sum, latencies, abstain_tp, abstain_fp) = results.iter()
    .fold(
        (0usize, 0, 0, 0.0f32, Vec::new(), 0, 0),
        |(r1, r5, r10, mrr, lats, tp, fp), r| {
            let new_tp = tp + (matches!(r.task_type, TaskType::Abstain) && r.abstained) as usize;
            let new_fp = fp + (!matches!(r.task_type, TaskType::Abstain) && r.abstained) as usize;
            (r1 + r.recall_at_1 as usize,
             r5 + r.recall_at_5 as usize,
             r10 + r.recall_at_10 as usize,
             mrr + r.reciprocal_rank,
             lats.into_iter().chain(std::iter::once(r.latency_ms)).collect(),
             new_tp, new_fp)
        }
    );
```

---

## Phase 3: Scorer Completeness (Priority: HIGH)

### 3.1 Implement NDCG@k (scorer.rs)

For multi-gold-evidence cases:
```rust
pub fn ndcg_at_k(case: &QueryCase, result: &CaseResult, k: usize) -> f32 {
    let gold: HashSet<&str> = case.gold_evidence_ids.iter().map(|s| s.as_str()).collect();
    let dcg: f32 = result.retrieved.iter()
        .take(k)
        .filter(|r| gold.contains(r.memory_id.as_str()))
        .enumerate()
        .map(|(i, _)| 1.0 / (2.0_f32.powi(i as i32 + 1)))
        .sum();

    let ideal_dcg: f32 = case.gold_evidence_ids.iter()
        .take(k)
        .enumerate()
        .map(|(i, _)| 1.0 / (2.0_f32.powi(i as i32 + 1)))
        .sum();

    if ideal_dcg > 0.0 { dcg / ideal_dcg } else { 0.0 }
}
```

### 3.2 HashSet for Gold Lookups (scorer.rs)

Already shown in 1.2 - convert gold_evidence_ids to HashSet once per call.

---

## Phase 4: Ingest Parallelization (Priority: MEDIUM)

### 4.1 Parallel Ingest Loop (runner.rs:32-38)

**Current**: Serial for loop
**Fix** - use futures buffered unordered:
```rust
use futures::stream::{self, StreamExt};

let ingest_futures: Vec<_> = sessions.iter()
    .flat_map(|s| s.turns.iter().filter_map(|t| {
        t.memory_id.as_ref().map(|id| adapter.ingest_memory(id, &t.text))
    }))
    .collect();

let results = stream::iter(ingest_futures)
    .buffer_unordered(4)  // Limit concurrent ops
    .collect::<Vec<_>>();
```

Requires `futures` crate in Cargo.toml.

### 4.2 Batch BM25 Insert (memory_adapter.rs)

Add batch method:
```rust
pub async fn ingest_batch(&self, items: &[(String, String)]) -> Result<()> {
    // Single BM25 lock acquisition
    {
        let mut bm25 = self.bm25_index.write().await;
        for (id, text) in items {
            bm25.add_document(id, &tokenize_for_bm25(text));
        }
    }
    // Then inject to framework in parallel via caller
    Ok(())
}
```

### 4.3 Borrowed Tokenization (memory_adapter.rs)

For query path only (indexing still needs owned Strings):
```rust
fn tokenize_for_bm25_query(text: &str) -> Vec<&str> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|s| s.len() > 1)
        .collect()
}
```

---

## Phase 5: Generator Improvements (Priority: MEDIUM)

### 5.1 Variable Session Length (generator.rs)

Add parameter:
```rust
pub fn generate_sessions(seed: u64, count: usize, turns_range: (usize, usize)) -> Vec<Session>
```

Randomize number of turns between min/max.

### 5.2 Association/MultiSession Cases (generator.rs)

**Association**: Cross-session linking queries
```rust
// Query that references concepts from two different sessions
QueryCase {
    query_id: format!("cross-assoc-{}", i),
    session_id: "cross-session".into(),
    task_type: TaskType::Association,
    query: "What color preference connects these sessions?".into(),
    gold_evidence_ids: vec![session_a_color_id, session_b_color_id],
    ...
}
```

**MultiSession**: Aggregation across sessions
```rust
QueryCase {
    query_id: format!("multi-{}", i),
    session_id: "multi-session".into(),
    task_type: TaskType::MultiSession,
    query: "How many different cities have I mentioned?".into(),
    gold_evidence_ids: all_city_ids,
    ...
}
```

---

## Phase 6: Measurement Accuracy (Priority: LOW)

### 6.1 Storage Bytes Estimation (runner.rs:121)

```rust
let sessions_path = cli.dataset_dir.join("sessions.jsonl");
let storage_bytes = std::fs::metadata(&sessions_path)
    .map(|m| m.len())
    .unwrap_or(0);
```

### 6.2 Tighten Latency Measurement (runner.rs:47-56)

Move timer closer to actual query:
```rust
let hdc_hits = {
    let start = Instant::now();
    let hits = self.framework.probe_text(text, top_k * 3).await?;
    latency_contrib.hdc_ns = start.elapsed().as_nanos();
    hits
};

let bm25_hits = {
    let start = Instant::now();
    let hits = self.bm25_index.read().await.search(&query_tokens, top_k * 3);
    latency_contrib.bm25_ns = start.elapsed().as_nanos();
    hits
};
```

### 6.3 Configurable Abstain Threshold (cli.rs, runner.rs)

Add CLI parameter:
```rust
#[arg(long, default_value = "0.1")]
pub abstain_threshold: f32,
```

Pass to runner and align with memory_adapter.rs HDC_MIN_SCORE.

---

## Execution Order (A* Optimal Path)

| Phase | Actions | Dependencies | Status |
|-------|---------|--------------|--------|
| 1 | Fix percentile, Defensive sort, sysinfo | None | Ready |
| 2 | p99, Task breakdown, Single-pass | Phase 1 | Blocked |
| 3 | NDCG@k, HashSet | None | Ready (parallel) |
| 4 | Parallel ingest, Batch BM25, Borrowed tokens | None | Ready (parallel) |
| 5 | Variable length, Cross-session | None | Ready (parallel) |
| 6 | Storage, Latency, CLI threshold | None | Ready (parallel) |

**Recommended approach**: Execute Phase 1 first (correctness), then Phase 2-6 can proceed in parallel.

---

## Test Requirements

All changes require:
- Unit tests for new metrics/scorer functions
- Integration test run with 100 sessions to verify correctness
- Benchmark comparison before/after to validate no regression

---

## Backward Compatibility

- New SummaryMetrics fields: use `#[serde(default)]`
- New TaskType cases: serde already handles
- New CLI parameters: defaults match current hardcoded values