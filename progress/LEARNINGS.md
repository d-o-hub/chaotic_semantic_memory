# LEARNINGS - Chaotic Semantic Memory

## Security Patterns
- **Path Hijacking (CWE-426)**: Resolve executables to absolute paths; filter PATH to exclude relative entries.
- **DoS Prevention**: Enforce bounds on public API params. Graph: `MAX_DEPTH=32`, `MAX_RESULTS=10K`. Batch: `max_batch_size=1000`.
- **Namespace Validation (CWE-770)**: 128-byte limit, non-empty, no control chars. Apply to any param that becomes a DB key.

## Performance Patterns
- **ILP over SIMD**: 4 independent accumulators in hot loops often beat SIMD (avoids STLF stalls).
- **Branchless bitmasks**: `w |= (cond as u128) << j` minimizes branch misprediction.
- **Zero-alloc interning**: `Arc<str>` + `get_mut`/`get_key_value` double-lookup for BM25 terms.
- **Rayon gating**: Parallelize only when N >= 32; scheduling overhead dominates small ops.
- **Bitmask modulo**: Power-of-2 buckets → `& (N-1)` instead of `% N`.
- **f32::min/max vs comparison operators**: `.min()`/`.max()` compile to single `llvm.minnum`/`llvm.maxnum` instructions — MORE vectorizable than if/else. Do NOT replace with `<`/`>` comparisons (reverses intentional mutation-test design, strips docs, adds exclusion debt).
- **Conversion elimination beats kernel tuning**: For BHVec10240::hamming, removing two full-array `to_hvec()` layout conversions (each a 1,280-byte copy) was the win (~2.6–2.75×), not new SIMD — the kernels already existed on the HVec path. Measure the full call path, not just the kernel.

## Baselines (x86_64)
| Operation | Latency |
|-----------|---------|
| HVec10240 hamming | ~219 ns |
| BHVec10240 hamming (direct dispatch, #597) | ~37.7 ns idle / ~54.5 ns loaded (~2.6–2.75× vs to_hvec path) |
| HVec10240 cosine | ~238 ns |
| Reservoir step 50k | ~136 µs |
| BM25 search 10k | ~406 µs |

## CI/CD Patterns
- **CI queue starvation**: GitHub Actions runners can leave runs "queued" indefinitely (>1hr). Release `wait-for-ci` must detect perpetual-queue and re-trigger via `gh run rerun`, not just wait. Add `timeout-minutes` to workflow jobs.
- **Concurrency on main**: `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}` — never cancel main pushes (cascades to release failures).
- **WASM dual-target**: `--target nodejs` for CI smoke tests (Node fetch can't do `file://`); `--target web` for release.
- **CJS/ESM interop**: `const exports = module.default || module;` before destructuring.
- **Cargo.lock atomicity**: Always commit lockfile with Cargo.toml changes or `--locked` jobs fail.
- **Node 20 deprecation**: Use actions supporting Node 24 (`checkout@v5+`, `rust-cache@v2.9.1+`).
- **Miri timeout**: 60 min minimum for ~220 tests.
- **Action pinning**: `git ls-remote --tags <url>` for exact SHA.
- **Native arm64 runners**: Cross-compiling NEON kernels is not testing them. `ubuntu-24.04-arm` runs `cargo test -p csm-core-lib` on real aarch64 hardware, exercising the NEON path against the scalar oracle (PR #599).
- **YAML plain-scalar trap**: `run: rustc -vV | grep 'host: aarch64'` — the `: ` inside single quotes still terminates a YAML plain scalar → workflow parse error. Double-quote the whole `run:` value.
- **Benchmark under load**: Absolute ns shift with machine load (idle vs harness-spin: 37.7 → 54.5 ns for the same binary). Report ratios measured under identical conditions, never bare absolute numbers.
- **Stale-binary detection**: cargo can silently reuse a previous build when shared target dirs flip between worktrees. A result that contradicts theory ("removing allocations made it 4.5x slower") means the wrong binary ran — verify the `Compiling csm-memory (path)` line in the build log before trusting any bench number.
- **Forced clean A/B**: `touch` the changed sources before each side's build and grep the log for which lib path was linked; interleave A/B/A/B and record `loadavg` before each run. Discard runs taken during load spikes (we excluded 281/301 us `main` runs taken at loadavg 2.7).
- **Deterministic test graphs**: hash-based pseudo-random edges (not ring/wrap successors) model association graphs realistically and reproduce identically across branches — required for a fair A/B (the ring graph's overlapping neighborhoods understated the BFS by ~7x).

## Codacy
- **`.codacy.yml` exclude_paths is the sanctioned unsafe-usage escape hatch**: SIMD hot paths with SAFETY comments belong in `engines.opengrep.exclude_paths` (repo policy) — NOT dashboard `AcceptedUse` suppressions, which are un-reviewable and vanish from the dashboard. Fix in code, or exclude per policy.
- **Safe-function restructure dead end**: Removing `#[target_feature]` from SIMD kernels forces every intrinsic call into its own `unsafe` block — MORE flagged sites, not fewer. Keep `unsafe fn` + `#[target_feature]`; exclude the file.

## Feature-Gating / Disabled-Capability Contracts (ADR-0094)
- **No false success on disabled features**: When a Cargo feature is off, optional builders must not silently drop config — record it and reject at `build()` with `UnsupportedOperation`; fallback facade methods must return `Err`, never `Ok`/empty.
- **Idempotent no-ops are fine**: `without_persistence()` when the feature is already off may return `self` (state already held, nothing discarded).
- **Gate tests, don't inherit fake success**: Integration tests exercising a disabled feature used to pass via the no-op stub. After the honest failure lands, every such test MUST get `#[cfg(feature = "persistence")]` per test or a `#![cfg(feature = "persistence")]` file gate — CI only ran `--all-features` and `--no-default-features --features ann-hnsw --lib`, so un-gated persistence tests never compiled in the lean matrix.
- **Verify the lean matrix compiles & passes**: `cargo test --no-default-features` (all targets) catches un-gated tests/examples that `--all-features` hides. Add `required-features` to examples and ad-hoc orphan examples.
- **Disk-full ≠ test failure**: `cc: No space left on device` during `cargo test --all-features` (60+ linked test binaries × huge optional deps) is environmental. `CARGO_PROFILE_TEST_DEBUG=0` shrinks binaries enough to fit; isolate the report.
- **Full-fidelity mutation coverage note**: `#[cfg(feature = "persistence")]` on a test also gates its doc — keep one rationale comment per assert.

## PR Triage / Jules Bot
- **Empty research PRs**: Close as no-op; zero file changes = no impact.
- **Commitlint full range**: `npx commitlint --from origin/main --to HEAD`. Invalid early scope fails CI even if later commits are fine.
- **Jules force-push risk**: Bot can rewrite PR after your fix, reverting sibling merges. Always `git diff origin/main...HEAD` before merge.
- **Merge order**: Independent green PRs first. Never `gh pr merge --auto` on stacks (rebase cancellation loop).
- **Mutation in-diff surface**: Cosmetic rewrites pull unrelated functions into cargo-mutants. Restore-to-main for unrelated lines.
- **`>` vs `>=` top-k**: Add test where `results.len() == top_k` so `>=` mutant panics.
- **CLI entry-point mutants**: `run_query -> Ok(())` unkillable under `--lib` mutation; exclude in `scripts/mutation_test.sh`.
- **`duplicated_attributes`**: Never `#![cfg(test)]` in a file also gated by `#[cfg(test)] mod` in lib.rs.

## Mutation Testing
- **Unreachable code = mutation smell**: Audit queue invariants when refactoring guards. Remove dead branches.
- **`--in-diff` on post-fix tree**: Generate diff after fix is staged, not before.
- **Cost**: ~14 min for 35-line diff, 11 mutants. Acceptable for PR validation.
- **New scopes**: Add to `commitlint.config.cjs` when creating workspace crates.

## Module-Specific
- **Reservoir**: CSR for >2000 nodes. Partitioned updates must preserve momentum.
- **Similarity**: Derive cosine from hamming (`1.0 - dist/5120.0`) for bipolar hvecs.
- **Top-K**: `select_nth_unstable_by` for O(N) partial sort.
- **WASM**: Gate rayon/IO with `#[cfg(not(target_arch = "wasm32"))]`.
- **Persistence**: `csm_`-prefixed tables. Update all surfaces (single, batch, export, WASM) when adding fields.
- **Floats**: Never `partial_cmp().unwrap()` — NaN panics. Use `total_cmp()`.

## State Management
- **Built ≠ Installed**: `~/.local/bin/csm` lags source. Always verify with `./target/debug/csm --help`.
- **GOAP_STATE drift**: Duplicate YAML keys silently overwritten. `grep -c '^  action_last_completed'` must equal 1.
- **ADR parity**: `scripts/check-adr-parity.sh` enforces registry ↔ disk sync.
- **Jules delegation**: `cost ≥ 12` actions → GitHub issue labeled `jules`, mark `status: delegated`.

## Supply Chain
- **`cargo deny check` before releases**: New advisories surface anytime. Maintain `deny.toml` ignore list.
- **Simple upgrades first**: `cargo update -p <pkg>` often resolves advisories without code changes.

## 2026-08-07 — Input Bound Validation on Decay Pruning
**Vulnerability:** The public API `prune_decayed_associations` accepted a float `threshold` parameter without any bounds or sanitization checks. Passing `NaN` as a threshold caused all associations to be silently deleted/pruned.
**Learning:** Even if helper validators exist (such as `validate_association_strength`), some public APIs may skip calling them because they don't perform direct inserts, failing to realize that downstream comparison operators (like `>=`) behave unexpectedly on malicious floats.
**Prevention:** Always validate all public API parameters of type `f32`/`f64` representing rates, scores, or thresholds against finite limits and expected ranges before performing state manipulation.

## 2026-08-07 — Enforce Upper Bounds on Character N-Gram Size
**Vulnerability:** `TextEncoder::encode_with_ngrams` accepted an unguarded `n: usize` parameter. Passing `n = usize::MAX` triggered `n + 1` integer overflow panics in `char_offsets.windows(n + 1)`.
**Learning:** Functions accepting `Option<usize>` or `usize` parameters on public APIs can bypass upper-bound checks if internal functions assume reasonable caller inputs without explicit constants.
**Prevention:** Always declare named upper-bound constants (`MAX_NGRAM_SIZE`) for sizing/windowing parameters and validate inputs before performing slice windowing or arithmetic additions.

## 2026-08-17 — Input Bound Clamping on Concept TTL
**Vulnerability:** Public builder API `ConceptBuilder::with_ttl` accepted arbitrary `u64` values without upper bounding, allowing arithmetic overflow when computing `now + ttl`.
**Learning:** Public builder methods taking time intervals or size limits must clamp input values to pre-defined maximum limits using saturating arithmetic.
**Prevention:** Enforce explicit `MAX_TTL_SECONDS_LIMIT` parameter bounds and saturating additions on all time-to-live public API builder interfaces.
