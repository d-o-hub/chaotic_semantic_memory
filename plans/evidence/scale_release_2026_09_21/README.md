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
| 10,000 | exact | 15.8 ms | 1.10 ms | 1.84 ms | 1.000 | 12.59 MB | — | 35.4 ms |
| 10,000 | hnsw | 14.49 s | 330 µs | 803 µs | 0.908 | 12.74 MB | 18.23 MB | 249.9 ms |
| 10,000 | lsh | 39.6 ms | 129 µs | 482 µs | 0.824 | 12.74 MB | 13.28 MB | 61.1 ms |
| 10,000 | bucket | 58.6 ms | 249 µs | 1.65 ms | 0.832 | 0 | — | — |
| 50,000 | exact | 84.6 ms | 4.31 ms | 7.06 ms | 1.000 | 62.94 MB | — | 174.6 ms |
| 50,000 | hnsw | 66.36 s | 728 µs | 1.07 ms | 0.892 | 63.71 MB | 92.86 MB | 1.49 s |
| 50,000 | lsh | 236.0 ms | 585 µs | 1.78 ms | 0.828 | 62.99 MB | 66.19 MB | 301.8 ms |
| 50,000 | bucket | 423.3 ms | 5.89 ms | 8.43 ms | 0.976 | 0 | — | — |
| 200,000 | exact | 347.4 ms | 19.41 ms | 25.39 ms | 1.000 | 251.77 MB | — | 754.9 ms |
| 200,000 | hnsw | 417.88 s | 1.27 ms | 2.64 ms | 0.724 | 254.82 MB | 380.38 MB | 8.80 s |
| 200,000 | lsh | 1.28 s | 2.10 ms | 4.22 ms | 0.832 | 250.30 MB | 264.40 MB | 1.40 s |
| 200,000 | bucket | 2.01 s | 24.38 ms | 33.93 ms | 1.000 | 0 | — | — |

Scaling trend (first → last scale):

| backend | build ×| p50 query ×| recall@10 first → last |
|---|---|---|---|
| exact | 21.9× | 17.7× | 1.000 → 1.000 ≈ |
| hnsw | 28.8× | 3.8× | 0.908 → 0.724 ↓ |
| lsh | 32.4× | 16.3× | 0.824 → 0.832 ≈ |
| bucket | 34.3× | 97.9× | 0.832 → 1.000 ↑ |

**HNSW recall falls with N at fixed `ef_search`** (0.908 → 0.724); raise `ef_search`/`m` for large corpora and re-measure before claiming a recall target.

Findings at the largest scale:

- **Exact scan is the query-latency wall**: 19.41 ms p50; HNSW answers in 1.27 ms (15.3× faster) for 0.724 recall, LSH in 2.10 ms for 0.832.
- **HNSW build is the cost, not the query**: 417.88 s to build against 1.28 s for LSH; reloading the serialized index (380.38 MB) takes 8.80 s.
- **Bucketed candidate generation (probe width 8) is not a recall-preserving filter**: 1.000 recall@10 at 24.38 ms p50 with 200000 candidates per query on average.
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

## Bucketed candidate probe sweep (`bucket_sweep/`)

`prefix_*` rows are the pre-fix generator (single bucket, fixed width, index-order truncation); `fixed_*` rows use the adaptive multi-probe and budget guard. `floor` is the configured `bucket_probe_width`, which the policy treats as a minimum.

| run | N | floor | candidates | recall@10 | exact fallbacks | queries | p50 |
|---|---|---|---|---|---|---|---|
| fixed_floor2_n10000 | 10,000 | 2 | 10,000 | 1.000 | 25 | 25 | 1.40 ms |
| fixed_floor2_n200000 | 200,000 | 2 | 200,000 | 1.000 | 25 | 25 | 23.10 ms |
| fixed_floor2_n50000 | 50,000 | 2 | 50,000 | 1.000 | 25 | 25 | 6.68 ms |
| fixed_floor8_n10000 | 10,000 | 8 | 570 | 0.832 | 0 | 25 | 259 µs |
| fixed_floor8_n200000 | 200,000 | 8 | 200,000 | 1.000 | 25 | 25 | 24.57 ms |
| fixed_floor8_n50000 | 50,000 | 8 | 46,080 | 0.976 | 23 | 25 | 5.17 ms |
| prefix_w12_n200000 | 200,000 | 12 | 903 | 0.292 | 0 | 25 | 2.01 ms |
| prefix_w16_n200000 | 200,000 | 16 | 810 | 0.260 | 0 | 25 | 1.64 ms |
| prefix_w2_n200000 | 200,000 | 2 | 1,000 | 0.016 | 0 | 25 | 3.58 ms |
| prefix_w4_n200000 | 200,000 | 4 | 1,000 | 0.068 | 0 | 25 | 2.58 ms |
| prefix_w6_n200000 | 200,000 | 6 | 1,000 | 0.144 | 0 | 25 | 2.32 ms |
| prefix_w8_n200000 | 200,000 | 8 | 967 | 0.208 | 0 | 25 | 2.99 ms |

Read together with the ANN table above: at `floor = 8` the probe is selective at 10 k (570 candidates, 0.832 recall, 2.2× faster than the exact scan) and declines at 50 k/200 k, where the corpus' prefix distribution makes the probe nearly as large as the corpus — the caller then scans exactly (recall 1.000) instead of returning the index-ordered slice that measured 0.016 recall before the fix.

## The bucketed-candidate recall fix (2026-09-21)

Pre-fix behaviour (single bucket, fixed width, index-order truncation) measured
recall@10 collapsing with corpus size — 0.604 at 10 k → 0.576 at 50 k → 0.208 at
200 k with `floor = 8`, and 0.016 at 200 k with the library default `floor = 2`
(the oversized bucket was cut to the first `max_candidates` indices, which is a
biased sample of the corpus). `fell_back_to_exact_scan` never fired.

The generator now:

1. **sizes the probe with the corpus** — `effective_bucket_probe_width` raises the
   configured width so one bucket fits `max_candidates` (clamped to
   `MAX_BUCKET_PROBE_WIDTH`);
2. **multi-probes** — candidates within one masked bit of the query bucket are
   accepted, recovering neighbours a single bucket loses
   (10 k: 0.604 → 0.832 at `floor = 8`);
3. **declines instead of truncating** — when the probe exceeds
   `max_candidates × 2`, it returns nothing and the caller performs the exact
   scan (`fell_back_to_exact_scan: true`), because an arbitrary slice of an
   oversized bucket is what produced the 0.016 recall.

Net effect at the scales measured: the bucketed path either returns a bounded,
higher-recall probe (10 k with `floor = 8`: 570 candidates, 0.832 recall,
249 µs vs 558 µs exact) or defers to the exact scan (50 k/200 k, recall 1.000 at
exact-scan latency). It can no longer silently return a low-recall candidate set.
