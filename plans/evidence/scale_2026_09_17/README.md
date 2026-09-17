# Scale evidence — 2026-09-17

ADR-0095 Tier-2 artifacts produced by `examples/scale_evidence` and
`scripts/scale-evidence.sh` on commit `4d81491` (clean tree), release profile,
rustc 1.88.0, Intel(R) Core(TM) i5-8350U CPU @ 1.70GHz, Linux x86_64.

Reproduce:

```bash
scripts/scale-evidence.sh ann         --out plans/evidence/scale_2026_09_17 --scales 1000,10000,50000 --queries 50
scripts/scale-evidence.sh persistence --out plans/evidence/scale_2026_09_17 --scales 1000,10000,50000 --tasks 8 --ops 25
scripts/scale-evidence.sh memory      --out plans/evidence/scale_2026_09_17 --fit 1000,5000,10000,50000 --holdout 100000
```

Each artifact carries its own manifest (`evidence_ann.json`,
`evidence_persistence.json`, `evidence_memory.json`) with the schema version,
commit, dirty state, command, features, corpus version/seed/checksum,
toolchain, hardware, sample count and reported variance statistics.

Corpus: `synthetic-clustered-v1` — 64 random cluster centres, concept `i` is
the centre for `i % 64` with 512 random bit flips, queries have 256 flips.
Generation is prefix-stable, so the first `k` concepts of an `N`-concept
corpus are identical for every `N`.

## ANN comparison (`ann_scale.json`)

`top_k = 10`, 50 queries per scale; recall@k is relevance-based (relevant
retrieved / total relevant) against the exact brute-force ground truth, not
hit rate.

| N | backend | build | p50 query | p99 query | recall@10 | index bytes | serialized | reload |
|---|---|---|---|---|---|---|---|---|
| 1,000 | exact | 2.0 ms | 58 µs | 978 µs | 1.000 | 1.26 MB | — | 4.2 ms |
| 1,000 | hnsw | 508.9 ms | 248 µs | 354 µs | 0.958 | 1.27 MB | 1.82 MB | 23.9 ms |
| 1,000 | lsh | 3.7 ms | 25 µs | 417 µs | 0.834 | 1.31 MB | 1.34 MB | 8.6 ms |
| 1,000 | bucket | 4.5 ms | 95 µs | 628 µs | 0.612 | 0 | — | — |
| 10,000 | exact | 17.7 ms | 758 µs | 2.08 ms | 1.000 | 12.59 MB | — | 37.2 ms |
| 10,000 | hnsw | 19.55 s | 530 µs | 1.29 ms | 0.922 | 12.74 MB | 18.38 MB | 410.6 ms |
| 10,000 | lsh | 73.1 ms | 187 µs | 3.50 ms | 0.810 | 12.74 MB | 13.28 MB | 79.5 ms |
| 10,000 | bucket | 78.4 ms | 478 µs | 3.91 ms | 0.554 | 0 | — | — |
| 50,000 | exact | 110.7 ms | 6.35 ms | 14.02 ms | 1.000 | 62.94 MB | — | 249.0 ms |
| 50,000 | hnsw | 80.15 s | 896 µs | 1.52 ms | 0.892 | 63.71 MB | 93.10 MB | 2.28 s |
| 50,000 | lsh | 282.1 ms | 813 µs | 4.88 ms | 0.836 | 62.99 MB | 66.19 MB | 496.1 ms |
| 50,000 | bucket | 655.6 ms | 1.83 ms | 17.17 ms | 0.590 | 0 | — | — |

Findings:

- **Exact scan is the query-latency wall**: 6.35 ms p50 at 50 k, growing with
  N. HNSW answers in 896 µs p50 at the same scale (7.1× faster) for 0.892
  recall; LSH answers in 813 µs for 0.836 recall.
- **HNSW build is the cost, not the query**: 80.15 s at 50 k against 282.1 ms for LSH.
  Reloading the serialized HNSW index (93.10 MB) takes 2.28 s, an order of
  magnitude below rebuilding.
- **Bucketed candidate generation with `bucket_probe_width = 8` is not a
  recall-preserving filter**: 0.590 recall@10 at 1.83 ms p50 (650 candidates per
  query on average). It is the fastest path but drops roughly 41 % of the true
  neighbours.
- Index bytes: HNSW adds 16 bytes/concept over the raw vectors, LSH 0;
  the bucketed path keeps no index at all. Serialized HNSW is 46 % larger
  than its in-memory footprint, LSH 5 %.

## Persistence (`persistence_scale.json`)

Batches of 500 concepts, DB/WAL/SHM measured separately:

| N | write throughput | batch p50 | read throughput | db bytes | wal | shm | bytes/concept |
|---|---|---|---|---|---|---|---|
| 1,000 | 5,479/s | 95.61 ms | 93,381/s | 2.81 MB | 0 | 0 | 2,949 |
| 10,000 | 5,133/s | 94.92 ms | 154,596/s | 27.23 MB | 0 | 0 | 2,855 |
| 50,000 | 5,185/s | 89.88 ms | 144,288/s | 135.97 MB | 0 | 0 | 2,851 |

Concurrent writers, 8 tasks × 25 round-trips on one local database:

| variant | ops/s | p50 | p95 | p99 | retries/op | error rate |
|---|---|---|---|---|---|---|
| raw (no retry) | 1,616 | 2.94 ms | 10.19 ms | 13.64 ms | 0.00 | **0.91** |
| caller-side bounded retry (5 × 2 ms) | 381 | 23.08 ms | 30.53 ms | 35.21 ms | 3.43 | **0.47** |

**Finding (ADR-0095 criterion unmet):** the persistence layer exposes no busy
timeout and no bounded retry, so 90 % of concurrent local writes fail; even a
caller-side bounded retry leaves 47 % failing. The evidence harness had to
implement the retry loop itself (as does `benches/persistence_benchmark.rs`).
Bounded retries/timeouts belong in `csm-persistence`; this artifact is the
before state for that change.

## Memory model (`memory_model.json`)

Per-point child processes; RSS delta is baseline-subtracted VmRSS, storage is
`db + wal + shm` after checkpoint.

| N | RSS delta | peak RSS | RSS/concept | storage | storage/concept |
|---|---|---|---|---|---|
| 1,000 | 5.83 MB | 10.82 MB | 6,111 B | 2.81 MB | 2,949 B |
| 5,000 | 25.01 MB | 29.14 MB | 5,245 B | 13.67 MB | 2,867 B |
| 10,000 | 49.31 MB | 54.30 MB | 5,170 B | 27.23 MB | 2,856 B |
| 50,000 | 226.11 MB | 230.14 MB | 4,742 B | 135.97 MB | 2,851 B |
| 100,000 (held out) | 451.45 MB | 455.47 MB | 4,734 B | 272.02 MB | 2,852 B |

Fits (points 1 k, 5 k, 10 k, 50 k):

```
RSS     = 2,880,665 B + 4,691.1 B × concepts    held-out error @100,000: 0.29 %
storage = 83,652 B + 2,849.7 B × concepts    held-out error @100,000: 0.06 %
```

10,000,000-concept projection: **43.69 GB RSS**, **26.54 GB storage** against the recorded
`< 12 MB` target — 3728× and 2265× over. `claim_supported: false`.

The 12 MB figure describes the product-quantization design (ADR-0024 phase 2:
~1 byte/concept + 2 MB codebook), which was never implemented. The shipped
representation is a 1 280-byte `HVec10240` held in memory (plus index copies)
and written twice per concept (concept row + version row). The formula-only
test in `tests/performance_targets.rs` asserts the arithmetic of the
unimplemented design and is replaced by this measurement in the follow-up
change.
