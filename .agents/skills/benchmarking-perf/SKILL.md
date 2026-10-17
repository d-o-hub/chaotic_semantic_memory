---
name: benchmarking-perf
description: "Run and analyze criterion benchmarks for performance-sensitive changes. Use when optimizing hot paths, validating perf targets, comparing baselines, or supplying the ADR-0095 '## Performance Evidence' section a perf(...) PR needs."
---

# Benchmarking & Performance

## Performance Targets

| Benchmark | Target | Current (v0.3.0) |
|---|---|---|
| `reservoir_step_50k` | < 100μs | ~57μs ✅ |
| `cosine_similarity` | < 1μs | ~0.15μs ✅ |
| `batch_similarity_1000` | < 500μs | ~280μs ✅ |
| `hvec_random` | < 5μs | ~0.49μs ✅ |
| `hvec_bind` | < 1μs | ~0.07μs ✅ |
| `bm25_search_10000` | < 5ms | ~2.2ms ✅ |
| `singularity_probe_50000` | < 10ms | ~3.7ms ✅ |

## Scalability Results

| Concept Count | Probe Time |
|--------------|------------|
| 100 | ~1.3ms |
| 1,000 | ~1.6ms |
| 10,000 | ~1.6ms |
| 50,000 | ~3.7ms |

Excellent scalability: 500x more concepts only 3x slower.

## Workflow

### 1. Save Baseline Before Changes
```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo bench --bench benchmark -- --save-baseline before 2>&1 | grep -E "(^test |time:|Benchmark|bench_|[0-9]+\.[0-9]+ µs)" | tail -30
```

### 2. Make Changes

### 3. Compare Against Baseline
```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo bench --bench benchmark -- --baseline before 2>&1 | grep -E "(time:|Benchmark|bench_|change:)" | tail -30
```

### 4. Interpret Results
- Look for `time: [lower upper]` in criterion output.
- Green = faster, Red = slower.
- Changes > 5% in hot paths require investigation.

## Adding a New Benchmark
Edit `benches/benchmark.rs`. Follow existing patterns:
```rust
fn bench_my_operation(c: &mut Criterion) {
    // Setup outside the closure
    let data = prepare_data();

    c.bench_function("my_operation", |b| {
        b.iter(|| {
            // Only the measured code here
            black_box(my_operation(black_box(&data)))
        })
    });
}
```
Add to `criterion_group!` at the bottom.

## Evidence for `perf(...)` PRs (ADR-0095 gate)

A PR whose title starts with `perf(` must document, **in the PR body**, three things: a baseline (a
benchmark id from `plans/evidence/bench/canonical.json`, the literal canonical path, or the nearest
canonical comparator plus the new bench output when the id has no entry), an artifact
(`target/criterion/<bench-id>/new/estimates.json`, a flamegraph path, or the CI artifact URL), and a
numeric before/after pair.

The gate is a **shape check** — it prints `shape OK — benchmark ids: …; artifact: …` and never runs
the bench, so proving reach is on you (next section). Dry-run it locally before touching GitHub:

```bash
python3 scripts/check-perf-pr-evidence.py "$PR_TITLE" "$(cat body.md)"
```

The old body must fail with the exact CI error and the new one print `shape OK`. Set the body
**before** the push that starts the run: `pull_request` fires on open/synchronize/reopen, a body edit
triggers nothing, and the gate reads the body captured in the run's payload.

Two artifact rules: never commit a rewritten `plans/evidence/bench/canonical.json` in the PR
(`scripts/bench-baseline.sh save` rewrites that committed file, and `compare` reports ids with no
baseline entry as "not in baseline"); and the numbers belong in the body, not only in commit messages.
Never retitle `perf(` → `refactor(` to make the gate skip — that is gate-gaming, not evidence.

## Prove the bench reaches the changed lines

An id can exist, be green, and still never execute the diff — check the callee's own guards, not the
function name. `make_candidates` prefills score `1.0` and `MmrReranker::rerank` early-returns above
`0.99`, so every `mmr_rerank/*` id timed the fast path; the loop only became measurable with a prefill
below the threshold (`mmr_rerank_slow/*`). A bench can also be blind because of its *arguments*
(`permute(321)`: `321 % 128 = 65`, so the word-aligned branch never runs).

- Prove reach with a marker, not by reading: an env-gated `std::sync::Once` + `eprintln!` in a scratch
  worktree prints the branch name once per process; run the bench with `--profile-time 1`.
- Prefer a new dedicated group over reparameterizing an existing id — that silently changes what the
  id measures for every other consumer.
- Case study and measured legs: the `bench-blindness` reference inside the `pr-roast-triage` skill.

## Measuring a diff: isolation and noise

- **One target dir per variant — never share `CARGO_TARGET_DIR` across worktrees.** Cargo treats a
  sibling worktree's artifacts as fresh: the second build prints `Finished … in 0.5 s` and reuses the
  *first* variant's binary (measured 2026-10-17). Before trusting any A/B, check distinct binary
  hashes and a `Compiling <crate>` line in each build log.
- Interleave legs (`A --save-baseline x` / `B --baseline x`, then the reverse) and require the sign to
  repeat. Name and exclude contaminated legs by their absolute times — a leg where *both* variants run
  ~2.4× slower is host load, not code — instead of averaging it in; record `uptime`.
- Size an `N - k`-operation saving with a `top_k = 1` probe: the whole pass becomes dead work (~5× the
  relative signal of the production shape) and it pins the marginal per-operation cost so the expected
  delta can be computed independently (2026-10-17: 350.2 → 317.0 µs at `k = 10` vs 77 → 50 µs at
  `k = 1`, ≈ 490 × 54 ns).
- Report medians with CIs, the host, the leg count, and any excluded leg — a single A/B pair on a
  shared host proves nothing.

## Gotchas
- Never `--baseline` without first `--save-baseline` with the same name.
- Don't capture mutable state by reference in criterion closures.
- Use `black_box()` on inputs AND outputs to prevent dead-code elimination.
- Reservoir benchmarks use `new_seeded(..., 42)` for reproducibility.
