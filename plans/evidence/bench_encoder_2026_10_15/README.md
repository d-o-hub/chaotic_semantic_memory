# Evidence — encoder single-token fast path (#897, 2026-10-15)

What was measured: `TextEncoder::encode` on a single token (`encode("hello")`) before
and after the fast path added by this PR to `crates/csm-core-lib/src/encoder.rs`, plus
the untouched three-token neighbour as a control.

**Files.** `{before,after}_<bench-id>.json` are verbatim copies of Criterion's
`target/criterion/<bench-id>/new/estimates.json` from each arm's tree.

**Arms.**

- *before*: a git worktree at `main` (`fe20979`) whose **only** change is
  `benches/embedding_benchmark.rs` — the benchmark file this PR adds — so the encoder
  under test is `main`'s slow path.
- *after*: this PR's branch.

**Commands**, run **serially, never concurrently**, same host, `CARGO_BUILD_JOBS=6`,
Criterion defaults (3 s warm-up, 100 samples, ~5 s measurement):

```
cd /tmp/base-encoder && cargo bench --bench embedding_benchmark -- 'encode_(single_1tok|short_3tok)'
cd <repo>          && cargo bench --bench embedding_benchmark -- 'encode_(single_1tok|short_3tok)'
```

| bench id | before — mean, 95 % CI | after — mean, 95 % CI |
|---|---|---|
| `encode_single_1tok` | 589.92 ns [589.39, 590.49] | 461.68 ns [460.82, 462.79] — **−21.7 %** |
| `encode_short_3tok` (control) | 3054.87 ns [3052.39, 3057.56] | 3078.30 ns [3075.07, 3081.94] — +0.8 % |

The control is the informative half: the untouched three-token path moved **up** 0.8 %
between the two arms, i.e. the host was marginally slower for the *after* arm — so the
single-token delta of −21.7 % is, if anything, conservative.

**Stability across repeats and protocols.** The after arm measured 471.53 ns in the
first run of this session and 461.27 ns in a second; round 1 measured the same code
independently with a raw harness (200 000 iterations/arm, 6 rounds, worktrees built
out of tree) at 590.6 ns → 468.3 ns. Same conclusion by three routes.

**Canonical baseline.** `plans/evidence/bench/canonical.json` (artifact
`criterion_baseline`, commit `ccb091b4`) has **no entry for `encode_single_1tok`**. Its
nearest comparator is `text_encoder/encode_short` (median 1597.4 ns), which comes from
a different benchmark harness and is therefore named for shape only — it is not treated
as a like-for-like baseline.
