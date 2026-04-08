# Chaotic Semantic Memory Benchmarks

This directory contains the `do-chaotic-semantic-memory-bench` crate, a reusable benchmark suite for evaluating chaotic semantic memory systems.

## Features

- **Deterministic seeded datasets**: Ensure reproducible results across runs.
- **Retrieval-first evaluation**: Focus on core memory performance without relying on expensive LLM calls.
- **Machine-readable outputs**: Results are saved in JSON, JSONL, and Markdown formats for easy tracking and comparison.
- **Low-cost and CI-friendly**: Designed to be run locally and in CI environments with minimal resource usage.

## Structure

- `src/`: Benchmark source code.
- `configs/`: Configuration files for different benchmark modes.
- `datasets/`: Versioned datasets for evaluation.
- `results/`: Output directory for benchmark reports and results.

## Usage

To run the default retrieval-only benchmark:

```bash
cargo run --release -- --mode retrieval-only
```

For more options, use the `--help` flag:

```bash
cargo run -- --help
```

## Principles

See [AGENTS.md](./AGENTS.md) for detailed principles and rules governing this benchmark suite.
