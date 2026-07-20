# GOAP Action Plan: Benchmark Suite Implementation (Issue #61)

## Current State (from GOAP_STATE.md)

```
project_initialized: true
core_modules_created: true
tests_passing: true
all_tests_passing: true
wasm_compiles: true
benchmarks_exist: true  # Main crate benchmarks only
ci_main_passing: true
```

## Target State

```
benchmark_suite_created: true
benchmarks_agents_md_exists: true
benchmark_crate_do_prefix: true
benchmark_code_under_benchmarks_dir: true
benchmark_ci_workflow_created: true
benchmark_ci_runs_success: true
benchmark_outputs_valid_json: true
benchmark_summary_json_written: true
benchmark_results_jsonl_written: true
benchmark_report_md_written: true
dataset_v1_seeded_deterministic: true
dataset_v1_small_exists: true
dataset_v1_medium_exists: true
retrieval_only_mode_works: true
zero_model_cost_default: true
reader_lite_mode_optional: true
no_production_shortcuts: true
results_comparable_across_commits: true
metrics_recall_at_k_implemented: true
metrics_mrr_implemented: true
metrics_abstention_implemented: true
metrics_latency_implemented: true
metrics_storage_memory_implemented: true
memory_adapter_in_memory_works: true
benchmark_cli_args_work: true
all_local_tests_pass: true
pr_created_and_merged: true
```

## Actions (Ordered by Cost/Effort)

### Action 1: Create benchmarks/AGENTS.md (cost: 2)
**Preconditions:** project_initialized
**Effects:** benchmarks_agents_md_exists

Create self-contained AGENTS.md in benchmarks/ directory with principles, required outputs, supported modes, dataset rules, coding rules, scoring rules, CI rules, and reporting rules.

---

### Action 2: Create benchmarks/Cargo.toml (cost: 2)
**Preconditions:** benchmarks_agents_md_exists
**Effects:** benchmark_crate_do_prefix, benchmark_code_under_benchmarks_dir

Create manifest with:
- name = "do-chaotic-semantic-memory-bench"
- path dependency on chaotic_semantic_memory
- dependencies: anyhow, clap, rand, rand_chacha, serde, serde_json, sysinfo, tokio, uuid, walkdir
- features: default=[], reader-lite=[]

---

### Action 3: Create benchmarks/src/types.rs (cost: 3)
**Preconditions:** benchmark_crate_do_prefix
**Effects:** metrics_types_defined

Define TaskType, SessionTurn, Session, QueryCase, RetrievedItem, CaseResult, SummaryMetrics structs with serde.

---

### Action 4: Create benchmarks/src/cli.rs (cost: 2)
**Preconditions:** metrics_types_defined
**Effects:** benchmark_cli_args_work

Define Args with clap: dataset_dir, out_dir, mode, top_k.

---

### Action 5: Create benchmarks/src/dataset.rs (cost: 2)
**Preconditions:** metrics_types_defined
**Effects:** dataset_loader_works

Implement load_sessions, load_queries from JSONL.

---

### Action 6: Create benchmarks/src/generator.rs (cost: 4)
**Preconditions:** dataset_loader_works
**Effects:** dataset_v1_seeded_deterministic

Implement generate_sessions(seed, count) and generate_queries(sessions) with ChaCha8Rng for deterministic output.

---

### Action 7: Create benchmarks/src/memory_adapter.rs (cost: 4)
**Preconditions:** dataset_v1_seeded_deterministic
**Effects:** memory_adapter_in_memory_works

Implement MemoryAdapter with in-memory ChaoticSemanticFramework, ingest_memory, query methods.

---

### Action 8: Create benchmarks/src/scorer.rs (cost: 3)
**Preconditions:** metrics_types_defined
**Effects:** metrics_recall_at_k_implemented, metrics_mrr_implemented

Implement hit_at_k, reciprocal_rank functions.

---

### Action 9: Create benchmarks/src/metrics.rs (cost: 4)
**Preconditions:** metrics_recall_at_k_implemented
**Effects:** metrics_abstention_implemented, metrics_latency_implemented, metrics_storage_memory_implemented

Implement aggregate_metrics: p50/p95 latency, abstention precision/recall, storage bytes, peak memory.

---

### Action 10: Create benchmarks/src/report.rs (cost: 3)
**Preconditions:** metrics_types_defined
**Effects:** benchmark_outputs_valid_json, benchmark_summary_json_written, benchmark_results_jsonl_written, benchmark_report_md_written

Implement write_summary, write_results_jsonl, write_markdown.

---

### Action 11: Create benchmarks/src/runner.rs (cost: 5)
**Preconditions:** memory_adapter_in_memory_works, metrics_types_defined, benchmark_cli_args_work
**Effects:** retrieval_only_mode_works, zero_model_cost_default

Orchestrate: load dataset → ingest → query → score → aggregate → write outputs.

---

### Action 12: Create benchmarks/src/main.rs (cost: 1)
**Preconditions:** runner module exists
**Effects:** benchmark_cli_works

Entry point with module declarations and runner::run call.

---

### Action 13: Create datasets/v1/seed files (cost: 3)
**Preconditions:** generator module works
**Effects:** dataset_v1_small_exists, dataset_v1_medium_exists

Write seeds.json, generate and write small/medium sessions.jsonl, queries.jsonl, manifest.json.

---

### Action 14: Create benchmarks/configs/ (cost: 2)
**Preconditions:** dataset_v1_small_exists
**Effects:** config_files_exist

Write ci-small.toml, retrieval-only.toml, reader-lite.toml.

---

### Action 15: Create .github/workflows/benchmark-ci.yml (cost: 3)
**Preconditions:** benchmark_cli_works, config_files_exist
**Effects:** benchmark_ci_workflow_created

Add workflow for small retrieval-only benchmark on PR/push.

---

### Action 16: Run and validate local benchmark (cost: 4)
**Preconditions:** benchmark_cli_works, dataset_v1_small_exists
**Effects:** all_local_tests_pass, results_comparable_across_commits

Execute: cargo run --manifest-path benchmarks/Cargo.toml, verify outputs.

---

### Action 17: Create PR and merge (cost: 3)
**Preconditions:** all_local_tests_pass, benchmark_ci_workflow_created
**Effects:** pr_created_and_merged, benchmark_suite_created

Atomic git commit, push, PR creation, verify CI passes, merge.

---

## Minimal Path Cost

Total: 41 (sum of action costs)

## Execution Order

1. AGENTS.md → Cargo.toml → types.rs → cli.rs → dataset.rs → generator.rs → memory_adapter.rs
2. scorer.rs → metrics.rs → report.rs → runner.rs → main.rs
3. datasets/v1/ → configs/ → workflow → validate → PR

## Parallelizable Actions

- After types.rs: cli.rs, dataset.rs can run in parallel
- After generator.rs: scorer.rs, memory_adapter.rs can run in parallel
- After metrics.rs: report.rs, datasets/ can run in parallel

## Risk Mitigation

- LOC gate: Keep each module under 200 lines (benchmarks directory exempt from 500-line cap)
- Integration: MemoryAdapter uses in-memory framework, no Turso dependency for CI
- Determinism: ChaCha8Rng with fixed seed ensures reproducible results

---

# Gap Analysis (Post-Implementation Review)

## TaskType Coverage

| TaskType | Defined | Generated | Coverage |
|----------|---------|-----------|----------|
| Recall | ✅ | ✅ | Full |
| Update | ✅ | ✅ | Partial (no version history) |
| Temporal | ✅ | ✅ | Minimal (no reservoir processing) |
| Abstain | ✅ | ✅ | Full |
| Association | ✅ | ❌ | **NOT TESTED** |
| MultiSession | ✅ | ❌ | **NOT IMPLEMENTED** |

## Critical Feature Gaps

| Feature | Codebase Support | Benchmark Coverage |
|---------|-----------------|-------------------|
| Semantic Bridge (ADR-0061) | `bridge_retrieval.rs` | ❌ None |
| BM25/Hybrid Retrieval (Issue #53) | `retrieval/bm25.rs` | ❌ None |
| TTL (Time-to-Live) | `framework_ttl.rs` | ❌ None |
| Version History | `framework_ops.rs:384-398` | ❌ None |
| Association Graph | `framework.rs:259-308` | ❌ Defined but NOT generated |

## Bugs Found

### Bug 1: Text Storage Metadata
**File**: `benchmarks/src/memory_adapter.rs:27-36`
- `inject_text()` does NOT store text in `_text` metadata
- `get_text()` always returns ID instead of original text
- **Fix**: Use `inject_text_with_metadata()` with `("_text", text)`

### Bug 2: Suboptimal Retrieval
**File**: `benchmarks/src/memory_adapter.rs:22-24`
- `probe_text()` uses pure HDC similarity
- For short queries (1-2 tokens), BM25 hybrid with 90% keyword weight is better
- **Fix**: Add hybrid retrieval mode

## API Coverage Summary

| API Category | APIs Tested | APIs Missing |
|--------------|-------------|--------------|
| Core | `inject_text`, `probe_text`, `get_concept` | `inject_text_with_ttl`, `probe_bridge_text` |
| Associations | None | `associate`, `get_associations`, `traverse` |
| Batch | None | `inject_concepts`, `probe_batch` |
| History | None | `concept_history` |
| Reservoir | None | `process_sequence` |

## Recommended Enhancements

1. **Immediate**: Fix text storage bug, generate Association test cases
2. **Short-term**: Add TTL tests, BM25/hybrid mode, version history tests
3. **Long-term**: Bridge retrieval mode, batch operation benchmarks, graph traversal tests