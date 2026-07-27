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

## Baselines (x86_64)
| Operation | Latency |
|-----------|---------|
| HVec10240 hamming | ~219 ns |
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
