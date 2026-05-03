# Benchmark Report

## Context
- Dataset: `benchmarks/datasets/v1/small` (version v1, seed 42, sessions 10)
- Mode: retrieval-only (reader mode: disabled)
- Retrieval top-k: 10 | Abstain threshold: 0.10
- Commit: be0a9ecc286d15f9140cd68007624b7ef0d35df0
## Outputs
- summary: `benchmarks/results/local/summary.json`
- results: `benchmarks/results/local/results.jsonl`
## Metrics
- Cases: 67
- Recall@1: 0.7308
- Recall@5: 0.7885
- Recall@10: 0.7885
- MRR: 0.7564
- NDCG@10: 0.6968
- Association Success: 0.4500
- Multi-session Recall: 1.0000
- Session Isolation: 0.0000
- Abstain precision: 0.4000
- Abstain recall: 0.6667
- Ingest ms: 4655
- p50 latency ms: 1
- p50 latency µs: 1555
- p95 latency ms: 6
- p99 latency ms: 7
- Storage bytes: 124526
- Peak memory bytes: 18087936
- Prompt tokens: 0
- Completion tokens: 0
