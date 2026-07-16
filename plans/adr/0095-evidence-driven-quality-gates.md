# ADR-0095: Evidence-Driven Quality Gates

## Status

Accepted (2026-07-16)

## Context and Problem Statement

The repository has extensive tests and a green main CI, but several recorded quality claims are not continuously enforced:

- no workflow runs Criterion benchmarks, `cargo fuzz`, `cargo deny`, benchmark-workspace unit tests, or the documented JS WASM smoke test;
- benchmark workflow path filters omit `crates/**` after workspace extraction;
- the fuzz workspace currently fails to compile due to public API drift;
- benchmark “Recall@k” is hit rate for multi-gold queries, NDCG uses exponential instead of logarithmic discount, and abstention truth is inferred from task type rather than the dataset label;
- the 10M-concept/12MB test asserts configured arithmetic rather than measuring implementation memory;
- ANN scale, persistence contention, and release performance claims lack one canonical reproducible artifact;
- mutation testing excludes high-risk areas and counts timeouts as caught;
- duplicated test sources inflate raw test counts.

A single always-on maximal gate would be prohibitively slow and hardware-sensitive, but the current mix permits unsupported claims.

## Decision Drivers

- Correctness gates must be deterministic and fail closed.
- Performance claims must be reproducible and tied to a commit/hardware profile.
- PR feedback must remain practical.
- Scale and remote-service evidence may run on schedules or release gates.
- Test counts and test/source LOC ratios are not substitutes for behavior or coverage.
- Supply-chain checks are mandatory before release.

## Considered Options

1. Keep current CI and treat benchmarks/fuzzing as local tools.
2. Run every test, fuzz target, and scale benchmark on every PR.
3. Adopt tiered evidence: PR correctness, scheduled scale, release claims.
4. Outsource all performance evidence to ad hoc manual reports.

## Decision Outcome

Chosen option: **three evidence tiers with a common machine-readable manifest and truthful metric definitions**.

### Tier 1 — required PR correctness

Run when relevant paths change:

- full compile/test/clippy/format matrix for all workspace packages;
- `cargo deny check --locked` or the supported locked equivalent;
- benchmark workspace unit tests;
- `cargo fuzz build` for all targets and a short run for changed targets;
- canonical WASM build plus Node/JS smoke test;
- targeted Criterion comparison for changed performance-sensitive crates on a controlled runner or non-blocking artifact lane until a stable runner is available;
- mutation testing for changed production files, with no silent module exclusion.

### Tier 2 — scheduled scale and resilience

Run nightly or weekly:

- all fuzz targets for a bounded duration with corpus/crash artifacts;
- exact, bucket, HNSW, and LSH retrieval at representative scales;
- index build/update/delete/serialize/reload evidence;
- local persistence read/write contention with bounded retries and timeouts;
- ignored DuckDB 1M-row and other expensive tests;
- full mutation sweep, including persistence, MCP, ANN serialization, and WASM-testable logic.

### Tier 3 — release claims

Before publishing a claim or release that changes relevant behavior:

- named reference runner and toolchain;
- release-sized ANN and memory/storage evidence;
- remote Turso evidence only in a credentialed, region-declared job;
- canonical Criterion baselines and confidence intervals;
- package-size and JS runtime smoke evidence for the exact npm artifact.

### Evidence manifest

Every performance artifact records commit SHA, dirty state, dataset/corpus version and checksum, seed, feature set, command, Rust/tool versions, OS/CPU, sample count, warmup, baseline, variance/confidence interval, and result schema version. GOAP booleans for performance targets may become true only with an artifact path produced for the current release lineage.

### Metric definitions

- Recall@k = relevant retrieved IDs / total relevant IDs; hit rate is reported separately.
- DCG discount = `1 / log2(rank + 1)` for 1-indexed rank.
- Abstention confusion matrices use `should_abstain`, not inferred task type.
- Memory reports baseline-subtracted peak RSS/allocator bytes and persisted DB/WAL/index bytes separately.
- Timeouts in mutation testing are unresolved failures, not killed mutants.
- Raw test-attribute count is inventory only. Coverage uses unique compiled behavior and line/branch coverage where practical.

## Positive Consequences

- Fast checks remain on PRs while scale evidence becomes credible.
- False or stale performance claims become detectable.
- Workspace extraction cannot bypass benchmark/fuzz triggers.
- Metric names match their mathematics.
- Release supply-chain requirements are continuously enforced.

## Negative Consequences

- Scheduled runners and artifact retention add cost.
- Performance thresholds need reference hardware or careful non-blocking rollout.
- Fuzz/mutation scope expansion may expose existing failures and increase runtime.
- Dataset and baseline governance require maintenance.

## Pros and Cons of the Options

### Local-only advanced quality tools

- Good, because CI is cheap.
- Bad, because the verified broken fuzz target and absent gates remain invisible.

### Everything on every PR

- Good, because feedback is immediate.
- Bad, because scale, remote, and full mutation jobs are too slow/noisy for every change.

### Tiered evidence

- Good, because cost and evidence strength are separated explicitly.
- Good, because release claims have stronger requirements than ordinary changes.
- Bad, because multiple workflows and artifact policies must remain synchronized.

### Manual reports

- Good, because setup is minimal.
- Bad, because results are easy to omit, age, or make irreproducible.

## TRIZ Rationale

- **Segmentation:** correctness, scale, and release-claim evidence are separate tiers.
- **Periodic action:** expensive fuzz/scale/mutation runs are scheduled.
- **Parameter change:** performance booleans become artifact references rather than unsupported assertions.

## Follow-up Actions

- `repair_fuzz_workspace_and_gate`
- `complete_workspace_ci_and_supply_chain_matrix`
- `correct_benchmark_metric_definitions`
- `establish_tiered_benchmark_evidence`
- `add_ann_and_persistence_scale_benchmarks`
- `replace_formula_only_memory_claim`
- `harden_mutation_evidence`
- `deduplicate_test_and_source_surfaces`

## Acceptance Criteria

- All fuzz targets build; changed targets run on PRs and all targets run on schedule with zero untriaged crashes/timeouts.
- Required CI includes every workspace package, cargo-deny, benchmark tests, and exact WASM JS smoke.
- Benchmark formulas pass hand-calculated multi-gold, duplicate, empty, and label-disagreement tests.
- Benchmark workflow triggers on owner crate paths.
- ANN evidence compares exact/bucket/HNSW/LSH and reports recall, query latency, build time, and bytes at agreed scales.
- Persistence concurrency has bounded retries/timeouts and reports p50/p95/p99, throughput, retry and error rates.
- Memory claims are based on measured points and bounded projection error, not constants alone.
- Mutation reports distinguish caught, missed, timeout, unviable, and excluded; changed production files have zero unexplained misses.
- Each published performance claim links a current evidence manifest.
