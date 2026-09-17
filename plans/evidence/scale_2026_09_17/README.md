# Scale evidence

ADR-0095 Tier-2 artifacts produced by `examples/scale_evidence` and
`scripts/scale-evidence.sh`; regenerate this file with
`python3 scripts/render-scale-evidence.py <this directory>`. Each artifact records
its own commit, dirty state, corpus checksum, toolchain and hardware in
`evidence_<mode>.json`.

Reference machine: Intel(R) Core(TM) i5-8350U CPU @ 1.70GHz, 8 threads, Linux x86_64.

Corpus: `synthetic-clustered-v1` — 64 random cluster centres, concept `i` is
the centre for `i % 64` with 512 random bit flips, queries have 256 flips.
Generation is prefix-stable, so the first `k` concepts of an `N`-concept
corpus are identical for every `N`.

## ANN comparison (`ann_scale.json`)

`top_k = 10`, 50 queries per scale; recall@k is
relevance-based (relevant retrieved / total relevant) against the exact
brute-force ground truth, not hit rate.

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

Findings at the largest scale:

- **Exact scan is the query-latency wall**: 6.35 ms p50; HNSW answers in 896 µs (7.1× faster) for 0.892 recall, LSH in 813 µs for 0.836.
- **HNSW build is the cost, not the query**: 80.15 s to build against 282.1 ms for LSH; reloading the serialized index (93.10 MB) takes 2.28 s.
- **Bucketed candidate generation (probe width 8) is not a recall-preserving filter**: 0.590 recall@10 at 1.83 ms p50 with 650 candidates per query on average.
- Index overhead over the raw vectors: HNSW 16 B/concept, LSH 0; the bucketed path keeps no index.

## Persistence (`persistence_scale.json`)

Batches of 500 concepts, DB/WAL/SHM measured separately:

| N | write throughput | batch p50 | read throughput | db bytes | wal | shm | bytes/concept |
|---|---|---|---|---|---|---|---|
| 1,000 | 7,793/s | 70.68 ms | 181,611/s | 2.81 MB | 0 | 0 | 2,949 |
| 10,000 | 8,733/s | 55.92 ms | 239,562/s | 27.23 MB | 0 | 0 | 2,855 |
| 50,000 | 8,450/s | 55.78 ms | 237,046/s | 135.97 MB | 0 | 0 | 2,851 |

Before the bounded retry/timeout change in `csm-persistence`
(`persistence_scale_pre_fix.json`, same workload):

pre-fix (8 tasks × 25 round-trips on one local database):

| variant | ops/s | p50 | p95 | p99 | retries/op | error rate |
|---|---|---|---|---|---|---|
| no retry | 1,616 | 2.94 ms | 10.19 ms | 13.64 ms | 0.00 | **0.905** |
| caller-side bounded retry (5 × 2 ms) | 381 | 23.08 ms | 30.53 ms | 35.21 ms | 3.43 | **0.470** |

post-fix (8 tasks × 25 round-trips on one local database):

| variant | ops/s | p50 | p95 | p99 | retries/op | error rate |
|---|---|---|---|---|---|---|
| no retry | 377 | 1.77 ms | 35.03 ms | 334.23 ms | 0.00 | **0.000** |
| caller-side bounded retry (5 × 2 ms) | 378 | 2.37 ms | 34.97 ms | 333.09 ms | 0.00 | **0.000** |

**Result:** with a 5 s busy timeout (and retries available when a wait is not enough) every concurrent write succeeds — error rate 0.905 → 0.000 raw and 0.470 → 0.000 with caller-side retries.

Writers now wait (bounded) instead of failing, so per-operation tail latency at high contention is higher while the failure rate is zero.

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

10,000,000-concept projection: **43.69 GB RSS**, **26.54 GB storage** against the recorded `< 12 MB` target — 3,728× and 2,265× over. `claim_supported: false`.

The 12 MB figure describes the product-quantization design (ADR-0024 phase 2:
~1 byte/concept + 2 MB codebook), which was never implemented. The shipped
representation is a 1 280-byte `HVec10240` held in memory (plus index copies)
and written twice per concept (concept row + version row).
