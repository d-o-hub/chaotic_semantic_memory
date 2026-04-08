# AGENTS.md

## Purpose

This directory contains a reusable benchmark workspace for memory-system evaluation.

The benchmark must measure retrieval quality, answer quality, abstention behavior, latency, storage cost, and token cost with deterministic reproducibility.

This AGENTS.md is intentionally local to `benchmarks/` and must be treated as independent from root-level project instructions unless a task explicitly requires root integration.

## Principles

- Prefer deterministic seeded datasets.
- Prefer retrieval-first evaluation.
- Prefer exact metrics over subjective grading.
- Prefer local and zero-cost execution by default.
- Prefer machine-readable outputs over screenshots or prose-only reports.
- Keep benchmark logic separate from production logic.
- Never optimize production code for a benchmark by adding benchmark-only shortcuts.
- Avoid hype, vanity metrics, and marketing phrasing.
- Minimize token usage in all benchmark flows.
- Default to no external model dependency.
- Use semantically meaningful test data (real words, not synthetic tokens like "color-5").

## Retrieval Strategy

The benchmark uses **hybrid retrieval** (BM25 + HDC) with query-length-dependent weights:

| Query Tokens | Keyword (BM25) | Semantic (HDC) | Rationale |
|-------------|----------------|----------------|-----------|
| 1-2 | 90% | 10% | Exact match dominates (function names, error strings) |
| 3-4 | 70% | 30% | Keywords still strong |
| 5-8 | 40% | 60% | Semantic takes over |
| 9+ | 20% | 80% | Full semantic mode (natural language questions) |

**Key insight**: For short queries under 5 tokens, keyword search consistently outperforms semantic search. Users searching for exact function names or error strings get poor results from embeddings alone.

**Implementation**: See `memory_adapter.rs` - uses `compute_weights()` and `merge_results()` from `retrieval/hybrid.rs`.

## Required outputs

Every benchmark run must write:
- `summary.json`
- `results.jsonl`
- `report.md`

Optional outputs:
- `reader_cases.jsonl`
- `latency_samples.jsonl`

All outputs must be written under `benchmarks/results/`.

## Supported modes

### retrieval-only
- No model calls.
- No judge LLM.
- Evaluate gold evidence retrieval only.

### reader-lite
- Small fixed subset only.
- Fixed prompt and config.
- Strict token caps.
- Exact-match or span-match scoring where possible.

## Dataset rules

- Datasets must be versioned under `benchmarks/datasets/`.
- Each dataset version must include a manifest and fixed seeds.
- Do not mutate an existing dataset version after baseline results are published.
- Add a new dataset version instead.

## Coding rules

- Keep each file focused on one responsibility.
- Avoid large files when practical.
- Keep interfaces stable and explicit.
- Prefer simple serde-based JSONL formats.
- Avoid hidden global state.
- Make benchmark runs deterministic where possible.
- Expose configs through CLI flags and TOML config files.
- Keep benchmark dependencies out of the production crate when possible.

## Scoring rules

Required:
- Recall@1
- Recall@5
- Recall@10
- MRR
- abstention precision
- abstention recall
- p50 latency
- p95 latency
- storage bytes
- peak memory bytes

Optional in reader mode:
- exact match
- span match
- prompt tokens
- completion tokens

## CI rules

- The small retrieval-only suite must be runnable in CI.
- CI must fail on malformed benchmark outputs or schema drift.
- Reader mode must not run in default CI unless explicitly enabled.

## Reporting rules

Reports must:
- avoid hype
- avoid leaderboard framing
- state dataset version
- state config profile
- state commit SHA when available
- state whether reader mode was enabled
- include raw metric values
- link to machine-readable outputs

## Change management

Any change to:
- metric definitions
- dataset schema
- config semantics
- reader prompt format
- report format

must be documented in a dedicated ADR or benchmark design note before merging.
