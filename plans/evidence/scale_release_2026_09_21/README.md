# Scale evidence

ADR-0095 scale-evidence artifacts produced by `examples/scale_evidence` and
`scripts/scale-evidence.sh`; regenerate this file with
`python3 scripts/render-scale-evidence.py <this directory's name>`. Each artifact
records its own commit, dirty state, corpus checksum, toolchain and hardware in
`evidence_<mode>.json`.

Reference machine: Intel(R) Core(TM) i5-8350U CPU @ 1.70GHz, 8 threads, Linux x86_64.

Corpus: `synthetic-clustered-v1` — 64 random cluster centres, concept `i` is
the centre for `i % 64` with 512 random bit flips, queries have 256 flips.
Generation is prefix-stable, so the first `k` concepts of an `N`-concept
corpus are identical for every `N`.

## ANN comparison (`ann_scale.json`)

`top_k = 10`, 25 queries per scale; recall@k is
relevance-based (relevant retrieved / total relevant) against the exact
brute-force ground truth, not hit rate.

| N | backend | build | p50 query | p99 query | recall@10 | index bytes | serialized | reload |
|---|---|---|---|---|---|---|---|---|
| 10,000 | exact | 11.9 ms | 558 µs | 1.43 ms | 1.000 | 12.59 MB | — | 35.1 ms |
| 10,000 | hnsw | 11.92 s | 490 µs | 678 µs | 0.908 | 12.74 MB | 18.47 MB | 229.2 ms |
| 10,000 | lsh | 31.5 ms | 136 µs | 302 µs | 0.824 | 12.74 MB | 13.28 MB | 63.0 ms |
| 10,000 | bucket | 45.8 ms | 191 µs | 689 µs | 0.604 | 0 | — | — |
| 50,000 | exact | 81.0 ms | 4.32 ms | 7.40 ms | 1.000 | 62.94 MB | — | 168.0 ms |
| 50,000 | hnsw | 56.11 s | 547 µs | 1.35 ms | 0.896 | 63.71 MB | 93.33 MB | 1.34 s |
| 50,000 | lsh | 207.0 ms | 433 µs | 778 µs | 0.832 | 62.99 MB | 66.19 MB | 295.6 ms |
| 50,000 | bucket | 358.3 ms | 464 µs | 1.58 ms | 0.576 | 0 | — | — |
| 200,000 | exact | 274.3 ms | 18.20 ms | 24.66 ms | 1.000 | 251.77 MB | — | 649.7 ms |
| 200,000 | hnsw | 330.14 s | 826 µs | 1.10 ms | 0.800 | 254.82 MB | 382.19 MB | 6.72 s |
| 200,000 | lsh | 858.5 ms | 1.86 ms | 2.94 ms | 0.820 | 250.30 MB | 264.40 MB | 1.40 s |
| 200,000 | bucket | 1.37 s | 1.74 ms | 3.82 ms | 0.208 | 0 | — | — |

Scaling trend (first → last scale):

| backend | build ×| p50 query ×| recall@10 first → last |
|---|---|---|---|
| exact | 23.1× | 32.6× | 1.000 → 1.000 ≈ |
| hnsw | 27.7× | 1.7× | 0.908 → 0.800 ↓ |
| lsh | 27.2× | 13.6× | 0.824 → 0.820 ≈ |
| bucket | 29.9× | 9.1× | 0.604 → 0.208 ↓ |

**Bucketed candidate recall degrades with N** (0.604 at 10,000 → 0.208 at 200,000): the probe mask (`bucket_probe_width`) is fixed while the corpus grows, so the candidate set stops covering the true neighbours. Scale the probe width with N or fall back to the exact scan above the size where recall matters.

**HNSW recall falls with N at fixed `ef_search`** (0.908 → 0.800); raise `ef_search`/`m` for large corpora and re-measure before claiming a recall target.

Findings at the largest scale:

- **Exact scan is the query-latency wall**: 18.20 ms p50; HNSW answers in 826 µs (22.0× faster) for 0.800 recall, LSH in 1.86 ms for 0.820.
- **HNSW build is the cost, not the query**: 330.14 s to build against 858.5 ms for LSH; reloading the serialized index (382.19 MB) takes 6.72 s.
- **Bucketed candidate generation (probe width 8) is not a recall-preserving filter**: 0.208 recall@10 at 1.74 ms p50 with 967 candidates per query on average.
- Index overhead over the raw vectors: HNSW 16 B/concept, LSH -8; the bucketed path keeps no index.

## Persistence (`persistence_scale.json`)

Batches of 500 concepts, DB/WAL/SHM measured separately:

| N | write throughput | batch p50 | read throughput | db bytes | wal | shm | bytes/concept |
|---|---|---|---|---|---|---|---|
| 50,000 | 9,132/s | 51.08 ms | 285,999/s | 135.97 MB | 0 | 0 | 2,851 |

post-fix (8 tasks × 25 round-trips on one local database):

| variant | ops/s | p50 | p95 | p99 | retries/op | error rate |
|---|---|---|---|---|---|---|
| no retry | 375 | 1.68 ms | 22.53 ms | 333.51 ms | 0.00 | **0.000** |
| caller-side bounded retry (5 × 2 ms) | 376 | 1.69 ms | 38.00 ms | 332.73 ms | 0.00 | **0.000** |

**Result:** error rate 0.000 raw, 0.000 with caller-side retries.

Writers now wait (bounded) instead of failing, so per-operation tail latency at high contention is higher while the failure rate is zero.

## Memory model (`memory_model.json`)

Per-point child processes; RSS delta is baseline-subtracted VmRSS, storage is
`db + wal + shm` after checkpoint.

| N | RSS delta | peak RSS | RSS/concept | storage | storage/concept |
|---|---|---|---|---|---|
| 50,000 | 226.19 MB | 229.98 MB | 4,744 B | 135.97 MB | 2,851 B |
| 100,000 | 451.48 MB | 455.58 MB | 4,734 B | 272.02 MB | 2,852 B |
| 200,000 | 902.47 MB | 906.39 MB | 4,732 B | 544.36 MB | 2,854 B |
| 500,000 (held out) | 2.61 GB | 3.14 GB | 5,604 B | 1.33 GB | 2,855 B |

Fits (points 50 k, 100 k, 200 k):

```
RSS     = 727,040 B + 4,727.8 B × concepts    held-out error @500,000: 15.62 %
storage = -215,040 B + 2,855.0 B × concepts    held-out error @500,000: 0.01 %
```

10,000,000-concept projection: **44.03 GB RSS**, **26.59 GB storage** against the recorded `< 12 MB` target — 3,757× and 2,269× over. `claim_supported: false`.

The 12 MB figure describes the product-quantization design (ADR-0024 phase 2:
~1 byte/concept + 2 MB codebook), which was never implemented. The shipped
representation is a 1 280-byte `HVec10240` held in memory (plus index copies)
and written twice per concept (concept row + version row).
