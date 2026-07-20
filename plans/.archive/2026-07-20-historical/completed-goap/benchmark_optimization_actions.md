# Benchmark Optimization Action Model

## Current State (2026-04-09)

```yaml
world_state:
  benchmark_suite_exists: true
  benchmark_tests_passing: true
  metrics_percentile_indexing_biased: true     # p50/p95 use non-standard indexing
  metrics_missing_p99: true                    # No p99_latency_ms field
  metrics_no_task_breakdown: true              # Flattened across all TaskTypes
  metrics_multi_pass_aggregation: true         # 6+ iterator passes in aggregate()
  scorer_missing_ndcg: true                    # No NDCG@k for multi-gold cases
  scorer_no_defensive_sort: true               # Assumes pre-sorted by rank
  scorer_nested_linear_scan: true              # gold_evidence_ids iteration
  ingest_sequential: true                      # Serial for loop
  ingest_redundant_locks: true                 # 2×N write lock acquisitions
  tokenize_allocates_vec: true                 # Vec<String> per call
  sysinfo_refresh_all_expensive: true          # refresh_all() vs refresh_process()
  storage_bytes_hardcoded_zero: true           # No actual storage tracking
  latency_includes_overhead: true              # RetrievedItem construction in timer
  sessions_fixed_length: true                  # All 3 turns, same structure
  association_multisession_not_generated: true # TaskTypes defined but unused
  abstain_threshold_hardcoded: true            # 0.1 vs HDC_MIN_SCORE 0.15
```

## Target State

```yaml
world_state:
  benchmark_suite_exists: true
  benchmark_tests_passing: true
  metrics_percentile_indexing_correct: true    # Standard floor-based indexing
  metrics_p99_present: true                    # p99_latency_ms field added
  metrics_task_breakdown: true                 # HashMap<TaskType, PartialMetrics>
  metrics_single_pass_aggregation: true        # One fold() for all stats
  scorer_ndcg_implemented: true                # NDCG@10 for multi-gold
  scorer_defensive_sort: true                  # Explicit sort guard
  scorer_hashset_optimization: true            # O(1) gold_evidence lookup
  ingest_parallel: true                        # Buffered unordered insert
  ingest_single_lock_batch: true               # One BM25 lock for all docs
  tokenize_borrowed_option: true               # &str slices for queries
  sysinfo_refresh_process: true                # PID-specific refresh
  storage_bytes_estimated: true                # File size or adapter footprint
  latency_precise: true                        # Tight timer around query
  sessions_variable_length: true               # Configurable turns_range
  association_multisession_generated: true    # Cross-session test cases
  abstain_threshold_configurable: true         # CLI parameter
```

## Actions

### A1: Fix Metrics Percentile Indexing
**Preconditions**: `metrics_percentile_indexing_biased`
**Effects**: `metrics_percentile_indexing_correct`, `metrics_percentile_indexing_biased = false`
**Cost**: 2 (simple arithmetic fix, no LOC impact)
**LOC**: ~3 lines in metrics.rs

```rust
// p50: use (count - 1) / 2 for true lower median
let p50 = latencies[(count - 1) / 2];
// p95: use floor not round
let p95_idx = ((count - 1) as f64 * 0.95) as usize; // floor via truncation
```

### A2: Add p99 Latency Metric
**Preconditions**: `metrics_missing_p99`, `metrics_percentile_indexing_correct`
**Effects**: `metrics_p99_present`, `metrics_missing_p99 = false`
**Cost**: 3 (add field to struct, update aggregate, update report)
**LOC**: ~10 lines across metrics.rs, types.rs, report.rs

### A3: Add Per-Task Metrics Breakdown
**Preconditions**: `metrics_no_task_breakdown`
**Effects**: `metrics_task_breakdown`, `metrics_no_task_breakdown = false`
**Cost**: 5 (HashMap structure, iteration per task)
**LOC**: ~15 lines in metrics.rs, types.rs

### A4: Single-Pass Aggregation
**Preconditions**: `metrics_multi_pass_aggregation`
**Effects**: `metrics_single_pass_aggregation`, `metrics_multi_pass_aggregation = false`
**Cost**: 4 (fold pattern refactoring)
**LOC**: ~20 lines in metrics.rs

### A5: Implement NDCG@k
**Preconditions**: `scorer_missing_ndcg`
**Effects**: `scorer_ndcg_implemented`, `scorer_missing_ndcg = false`
**Cost**: 6 (new function, logarithmic discount)
**LOC**: ~15 lines in scorer.rs

### A6: Add Defensive Sort Guard
**Preconditions**: `scorer_no_defensive_sort`
**Effects**: `scorer_defensive_sort`, `scorer_no_defensive_sort = false`
**Cost**: 2 (single line sort_unstable_by_key)
**LOC**: ~1 line in scorer.rs

### A7: HashSet Optimization for Gold Lookups
**Preconditions**: `scorer_nested_linear_scan`
**Effects**: `scorer_hashset_optimization`, `scorer_nested_linear_scan = false`
**Cost**: 3 (HashSet creation per case)
**LOC**: ~5 lines in scorer.rs

### A8: Parallelize Ingest with Buffered Unordered
**Preconditions**: `ingest_sequential`
**Effects**: `ingest_parallel`, `ingest_sequential = false`
**Cost**: 8 (futures stream, buffer_unordered, error handling)
**LOC**: ~25 lines in runner.rs
**Dependencies**: requires `futures` crate

### A9: Batch BM25 Insert Under Single Lock
**Preconditions**: `ingest_redundant_locks`, `ingest_parallel`
**Effects**: `ingest_single_lock_batch`, `ingest_redundant_locks = false`
**Cost**: 5 (collect pairs, single write lock)
**LOC**: ~15 lines in memory_adapter.rs

### A10: Borrowed Tokenization for BM25 Queries
**Preconditions**: `tokenize_allocates_vec`
**Effects**: `tokenize_borrowed_option`, `tokenize_allocates_vec = false`
**Cost**: 4 (new function, update callers)
**LOC**: ~10 lines in memory_adapter.rs

### A11: Fix sysinfo Sampling
**Preconditions**: `sysinfo_refresh_all_expensive`
**Effects**: `sysinfo_refresh_process`, `sysinfo_refresh_all_expensive = false`
**Cost**: 3 (refresh_process in query loop)
**LOC**: ~5 lines in runner.rs

### A12: Estimate Storage Bytes
**Preconditions**: `storage_bytes_hardcoded_zero`
**Effects**: `storage_bytes_estimated`, `storage_bytes_hardcoded_zero = false`
**Cost**: 4 (file size read or adapter method)
**LOC**: ~10 lines in runner.rs, memory_adapter.rs

### A13: Tighten Latency Measurement
**Preconditions**: `latency_includes_overhead`
**Effects**: `latency_precise`, `latency_includes_overhead = false`
**Cost**: 3 (move timer start/stop)
**LOC**: ~5 lines in runner.rs

### A14: Variable Session Length
**Preconditions**: `sessions_fixed_length`
**Effects**: `sessions_variable_length`, `sessions_fixed_length = false`
**Cost**: 5 (turns_range parameter, rng selection)
**LOC**: ~15 lines in generator.rs

### A15: Generate Association/MultiSession Cases
**Preconditions**: `association_multisession_not_generated`
**Effects**: `association_multisession_generated`, `association_multisession_not_generated = false`
**Cost**: 7 (cross-session ID generation, query templates)
**LOC**: ~20 lines in generator.rs

### A16: Configurable Abstain Threshold
**Preconditions**: `abstain_threshold_hardcoded`
**Effects**: `abstain_threshold_configurable`, `abstain_threshold_hardcoded = false`
**Cost**: 3 (CLI parameter, alignment with HDC_MIN_SCORE)
**LOC**: ~5 lines in cli.rs, runner.rs

## Optimal Path (A* heuristic)

**Phase 1: Low-cost correctness fixes (cost ~12)**
- A1 (p50/p95 fix) → A6 (defensive sort) → A7 (HashSet) → A11 (sysinfo)

**Phase 2: Metrics enhancements (cost ~14)**
- A2 (p99) → A3 (task breakdown) → A4 (single-pass)

**Phase 3: Scorer completeness (cost ~6)**
- A5 (NDCG@k) — independent, can run parallel to Phase 2

**Phase 4: Ingest parallelization (cost ~17)**
- A8 (parallel) → A9 (batch) → A10 (borrowed tokens)

**Phase 5: Generator improvements (cost ~15)**
- A14 (variable length) → A15 (cross-session types)

**Phase 6: Measurement accuracy (cost ~7)**
- A12 (storage) → A13 (latency) → A16 (abstain CLI)

**Total estimated cost**: ~51 action units
**Estimated LOC impact**: ~150 lines across 6 files

## Hard Constraints

- **LOC gate**: 500 lines max per file (all actions must respect)
- **Test coverage**: All new metrics/scorer functions need unit tests
- **Backward compatibility**: New SummaryMetrics fields must use `#[serde(default)]`
- **Feature gates**: Parallel ingest requires `tokio::rt-multi-thread` feature

## Risk Assessment

| Action | Risk | Mitigation |
|--------|------|------------|
| A8 (parallel ingest) | Medium - error handling in concurrent context | Use `buffer_unordered(4)` for bounded parallelism |
| A9 (batch BM25) | Low - single lock reduces contention | Test with 10k sessions |
| A15 (cross-session) | Medium - complex query templates | Start with simple Association cases |