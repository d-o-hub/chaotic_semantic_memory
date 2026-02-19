---
description: "Run and analyze criterion benchmarks for performance-sensitive changes. Use when optimizing hot paths, validating perf targets, or comparing baselines."
mode: subagent
tools:
  write: true
  edit: true
  bash: true
  glob: true
  grep: true
  read: true
  skill: true
---
# perf Agent

This agent combines multiple skills for efficient workflow.

## Skills Used

- benchmarking-perf
- debugging-reservoir
- swarm-performance

## How to Use

- **@perf**: Invoke this agent for combined workflow
- Automatically loads relevant skills based on task

## Skill Details

### benchmarking-perf
Run and analyze criterion benchmarks for performance-sensitive changes. Use when optimizing hot paths, validating perf targets, or comparing baselines.

### debugging-reservoir
Debug and tune the echo state network reservoir. Use when diagnosing spectral radius issues, chaotic dynamics problems, sparse weight anomalies, or reservoir-to-hypervector projection failures.

### swarm-performance
SIMD optimization, connection pooling, batch APIs, and caching. Use when improving throughput or reducing latency.

## Generated

This file is auto-generated from skill mappings.
Run `scripts/generate-agents.sh` to regenerate.
