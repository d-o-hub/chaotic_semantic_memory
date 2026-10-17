# Blind benches and the MMR final-step short-circuit (2026-10-17, #909 / closed #891)

## The change under roast

`MmrReranker::rerank` (`crates/csm-retrieval/src/rerank.rs`) updates the
`max_sim_to_selected` cache after every selection. On the last iteration the
updated cache is never read again, so #909 guards the update with
`selected.len() + 1 < top_k` — the loop's own continuation condition, evaluated
for the post-push length (`selected.push` sits *after* the update block). The
saving is `N - k` `cosine_similarity` calls: for `N = 500, k = 10` that is 490
of the 4945 pairwise-update sims (9.9 %) and 490 of the 5445 total sims (9.0 %).

## Why the existing bench cannot see it

- `benches/rerank_benchmark.rs:19` — `score: 1.0 - (i as f32 / n as f32)` gives
  candidate 0 a prefill score of exactly `1.0`.
- `crates/csm-retrieval/src/rerank.rs:57` — `rerank` returns early when
  `self.lambda >= 1.0 || candidates.first().is_some_and(|c| c.score >= 0.99)`,
  so the selection loop (and the changed lines) never run.
- Marker proof, in a scratch worktree (`eprintln!` behind `CSM_MMR_TRACE`,
  wrapped in `std::sync::Once` so each branch prints at most once per process):
  - filter `mmr_rerank/|mmr_rerank_pure/` (all six existing ids) → `fast_path`
    only;
  - filter `mmr_rerank_slow/` with prefilled `score = 0.9 - i/10_000` →
    `slow_path`.
- So `mmr_rerank/top10_from_500` — the id #891 cited for
  "60.24 → 56.47 µs (~6.2 % speedup)" — times the fast path. Those numbers are
  not attributable to the diff. Two control legs on that id, with the diff in
  place, read −3.5 % (p = 0.38) then +10.2 % (p = 0.01, "regressed"): noise with
  no repeatable sign, on code the diff does not touch.

## Making it measurable

Reparameterize the prefill so the loop runs (a PR adding this for real should
either change `make_candidates` or add the group):

```rust
// candidate 0 must stay below 0.99 or the fast path swallows the loop
let candidates: Vec<RerankCandidate> = make_candidates(size)
    .into_iter()
    .enumerate()
    .map(|(i, mut cand)| {
        cand.score = 0.9 - (i as f32 / 10_000.0);
        cand
    })
    .collect();
```

Size the per-unit cost with a `top_k = 1` variant: the scoring loop and the
single selection are unchanged, but the whole update pass becomes dead work, so
the delta is `N - 1` sims — roughly 5× the relative signal of `N = 500, k = 10`.

## Measured legs

Criterion, `taskset -c 2`, `--warm-up-time 1 --measurement-time 4 --sample-size 40`,
legs alternating `A --save-baseline` / `B --baseline` so each compared pair flips
order. Host: the `canonical.json` reference CPU (`Intel i5-8350U`), while a
foreign `cargo-nextest` build held ~7 `rustc` processes (load 5–9).

| bench | leg 1 | leg 2 | leg 3 | leg 4 |
|---|---|---|---|---|
| `mmr_rerank_slow_k1/top1_from_500` | −34.6 % (p 0.00) | +79.5 % base-vs-head (p 0.00) | −36.9 % (p 0.00) | +46.5 % base-vs-head (p 0.00) |
| `mmr_rerank_slow/top10_from_500` | −8.8 % (p 0.00) | +41.5 % base-vs-head (p 0.00) | −17.3 % (p 0.00) | +4.9 % (p 0.42) |

Leg 4 of the `k = 10` set is contaminated — both variants read ~1.0–1.1 ms
(≈ 2.4× the clean legs), and p = 0.42 — so it is named and excluded rather than
averaged in. `top_k = 1` medians: base 85.7 / 95.3 / 86.1 / 88.8 µs vs head
55.6 / 51.8 / 54.7 / 54.4 µs (≈ 40 % faster), implying ≈ 64 ns per in-workload
sim (`(86 − 54) µs / 499`) and therefore ≈ 31 µs of saving at `k = 10` against
410–460 µs baselines. The clean `k = 10` legs read −8.8 % / −17.3 % (criterion)
with per-leg mean ratios −9.3 % / −12.9 % / −13.9 %, the same order.

## Correctness pin (measured, not argued)

Mutating the guard to `selected.len() + 2 < top_k` — the stale-cache direction
that also skips the `i = k-1` update — makes `cargo test -p csm-retrieval --lib`
fail, 2 of 58: `rerank::tests::test_mmr_reranker` (`rerank_tests.rs:56`,
`results[1].id`) and
`rerank::tests::test_mmr_lambda_zero_score_is_negative_after_first_selection`
(`:193`). Control run with the guard restored (markers still compiled in):
58 passed / 0 failed. The framework pipeline test (`framework_rerank_tests.rs:60`)
does *not* catch it: `probe_with_rerankers` (`src/framework_rerank.rs:55`) passes
`actual_initial_k = initial_top_k.max(final_top_k)` to `rerank` and truncates to
`final_top_k` afterwards, so a changed 3rd pick is discarded — only the
crate-level unit tests observe the k-th selection. Equivalent mutants (skip
nothing, always skip) are unobservable by construction, so the mutation gate can
never *score* this optimization; only a benchmark can.
