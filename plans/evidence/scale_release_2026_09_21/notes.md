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
