# Benchmark Report

## Context
- Dataset: `benchmarks/datasets/v1/small` (version v1, seed 42, sessions 10)
- Mode: retrieval-only (reader mode: disabled)
- Retrieval top-k: 10 | Abstain threshold: 0.10
- Commit: 3ee2ca90951ab0042dab49dca0874c202ad457ef
## Outputs
- summary: `benchmarks/results/local/summary.json`
- results: `benchmarks/results/local/results.jsonl`
## Metrics
- Cases: 40
- Recall@1: 0.7500
- Recall@5: 0.7500
- Recall@10: 0.7500
- MRR: 0.7500
- NDCG@10: 0.7500
- Abstain precision: 1.0000
- Abstain recall: 1.0000
- Ingest ms: 17
- p50 latency ms: 0
- p50 latency µs: 806
- p95 latency ms: 1
- p99 latency ms: 1
- Storage bytes: 27254
- Peak memory bytes: 13238272
- Prompt tokens: 0
- Completion tokens: 0
