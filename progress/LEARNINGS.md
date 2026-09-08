# LEARNINGS - Chaotic Semantic Memory

> **Compacted 2026-09-08**: dated narrative entries folded into topical sections; full
> narrative history lives in git and `plans/PR_ROAST_*.md`. One tight bullet per
> distinct lesson; new lessons append to the matching section, not new dated blocks.

## Security Patterns
- **Path hijacking (CWE-426)**: resolve executables to absolute paths; filter PATH to exclude relative entries.
- **Public API input bounds (2026-08-07/17 cluster)**: NaN `threshold` in `prune_decayed_associations` silently deleted all associations; unguarded `n: usize` in `encode_with_ngrams` overflowed `windows(n+1)`; unbounded `ConceptBuilder::with_ttl` overflowed `now + ttl`. Prevention: finite-limit validation on all public f32/f64 rate/score/threshold params (even when helpers like `validate_association_strength` exist — non-inserting APIs skip them); named `MAX_*` constants for usize size/window params; saturating arithmetic on all time/size intervals.
- **Namespace validation (CWE-770)**: 128-byte limit, non-empty, no control chars — apply to any param that becomes a DB key.
- **DoS bounds**: Graph `MAX_DEPTH=32`, `MAX_RESULTS=10K`; batch `max_batch_size=1000`.

## Performance Patterns
- **ILP over SIMD**: 4 independent accumulators in hot loops often beat SIMD (avoids STLF stalls).
- **Branchless bitmasks**: `w |= (cond as u128) << j` minimizes branch misprediction.
- **Zero-alloc interning**: `Arc<str>` + `get_mut`/`get_key_value` double-lookup for BM25 terms; `HashSet<&str>` + single-pass `insert()` kills the contains+insert double lookup (PR #679).
- **Rayon gating**: parallelize only when N >= 32; scheduling overhead dominates small ops.
- **Bitmask modulo**: power-of-2 buckets → `& (N-1)` instead of `% N`.
- **f32::min/max vs operators**: `.min()`/`.max()` compile to `llvm.minnum`/`llvm.maxnum` — MORE vectorizable than if/else. Do NOT "simplify" to `<`/`>` (reverses mutation-test design, strips docs, adds exclusion debt).
- **Conversion elimination beats kernel tuning**: BHVec10240::hamming's ~2.6–2.75× came from removing two 1,280-byte `to_hvec()` copies, not new SIMD. Measure the full call path, not just the kernel.

## Baselines (x86_64)
| Operation | Latency |
|-----------|---------|
| HVec10240 hamming | ~219 ns |
| BHVec10240 hamming (direct dispatch, #597) | ~37.7 ns idle / ~54.5 ns loaded (~2.6–2.75× vs to_hvec path) |
| HVec10240 cosine | ~238 ns |
| Reservoir step 50k | ~136 µs |
| BM25 search 10k | ~406 µs |


## CI/CD Patterns
- **gh multi-account (2026-09-08)**: keyring can hold `d-oit` + `d-o-hub`; after `gh auth switch` the CLI may still 401 on GraphQL. Verify identity with `gh api graphql -f query='{viewer{login}}'` and pin operations with `export GH_TOKEN=$(gh auth token -u d-o-hub)`. Dependabot silently ignores commands from accounts without write access — post `@dependabot rebase` as the repo owner.
- **Branch sync without `gh pr update-branch`** (subcommand absent in installed gh): `gh api -X PUT repos/<owner>/<repo>/pulls/<n>/update-branch`.
- **Dependabot rebase ordering (2026-09-08)**: batch `@dependabot rebase` comments only AFTER all human PRs land (single rebase onto final main). Cargo.lock-group rebases lag workflow-file rebases (dependency resolution); drafts must be `gh pr ready` before any merge attempt.
- **CI queue starvation**: runs can sit "queued" >1hr; release `wait-for-ci` must detect perpetual-queue and `gh run rerun`. Add `timeout-minutes` to jobs.
- **Concurrency on main**: `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}` — never cancel main pushes.
- **WASM dual-target**: `--target nodejs` for CI smoke, `--target web` for release.
- **CJS/ESM interop**: `const exports = module.default || module;` before destructuring.
- **Cargo.lock atomicity**: always commit lockfile with Cargo.toml changes or `--locked` jobs fail.
- **Node 20 deprecation**: use Node 24-capable actions (`checkout@v5+`, `rust-cache@v2.9.1+`).
- **Miri timeout**: 60 min minimum for ~220 tests.
- **Action pinning**: `git ls-remote --tags <url>` for exact SHA.
- **Native arm64 runners**: cross-compiled NEON is not tested NEON; `ubuntu-24.04-arm` runs `cargo test -p csm-core-lib` on real aarch64 (PR #599).
- **YAML plain-scalar trap**: `: ` inside single quotes still terminates a plain scalar — double-quote the whole `run:` value.
- **Benchmark under load**: absolute ns shift with load (37.7 → 54.5 ns idle→loaded). Report ratios under identical conditions, never bare absolutes.
- **Stale-binary detection**: cargo silently reuses builds across worktrees sharing a target dir; a result contradicting theory means the wrong binary ran — grep the log for `Compiling csm-memory (path)` before trusting a number.
- **Forced clean A/B**: `touch` changed sources per side, grep the linked lib path, interleave A/B/A/B, record `loadavg`, discard runs during spikes (excluded 281/301 runs at loadavg 2.7).
- **Deterministic test graphs**: hash-based pseudo-random edges (not ring/wrap successors) model association graphs realistically and reproduce across branches — ring graphs understated BFS wins ~7×.

## Codacy
- **`.codacy.yml` `exclude_paths` is the sanctioned unsafe escape hatch**: SIMD hot paths with SAFETY comments go in `engines.opengrep.exclude_paths` — never dashboard `AcceptedUse` suppressions (un-reviewable, vanish from dashboard).
- **Safe-function restructure dead end**: removing `#[target_feature]` forces every intrinsic call into its own `unsafe` block — MORE flagged sites. Keep `unsafe fn` + `#[target_feature]`; exclude the file.

## Feature-Gating / Disabled-Capability Contracts (ADR-0094)
- **No false success on disabled features**: record config and reject at `build()` with `UnsupportedOperation`; fallback facades return `Err`, never `Ok`/empty. Idempotent no-ops (`without_persistence()` when already off → `self`) are fine.
- **Gate tests, don't inherit fake success**: every test exercising a disabled feature needs per-test `#[cfg(feature = "persistence")]` or a file gate — CI's `--all-features` and lean matrix never compiled un-gated tests. Verify with full `cargo test --no-default-features` (all targets) + `required-features` on examples.
- **Disk-full ≠ test failure**: `cc: No space left on device` under `--all-features` is environmental; `CARGO_PROFILE_TEST_DEBUG=0` shrinks linked test binaries.
- **Mutation note**: `#[cfg(feature)]` on a test also gates its doc — keep one rationale comment per assert.

## PR Triage / Jules Bot
- **No impact = close**: empty research PRs (zero file changes) close as no-op; duplicates close as `superseded by #<keeper>`.
- **Duplicate swarm detection (2026-09-04)**: bots file near-identical PRs (same base blob + hunk). Detect via `gh pr diff` overlap; keep one (green > mergeable, fewest unrelated files, compat wrapper kept, newest branch). Reusable workflow: `.agents/skills/pr-roast-triage/SKILL.md`.
- **Draft gate (2026-09-08)**: Jules PRs arrive as drafts — merge fails with `mergePullRequest: still a draft`. Check `isDraft` + `mergeStateStatus` (`BEHIND`/`BLOCKED`/`CLEAN`) in every triage pass.
- **Title+body honesty pair (2026-09-08)**: retitling without rewriting the body leaves an honest title over a dishonest description — full body rewrite via REST (`gh api -X PATCH .../pulls/<n> -f body=...`) in the same pass. Verify the real delta with `gh pr diff <n> --name-only`; a stale local `origin/main` phantom-adds files to three-dot `git diff` stats — trust the API's `changed_files`.
- **Doc-comment stripping (2026-09-07)**: near the 500 LOC gate, bots strip docstrings to fit code. Correct pattern: extract child submodules (`hyperdim_binary_serde.rs`, PR #670), preserving 100% of comments.
- **GraphQL deprecation**: `gh pr edit`/`gh issue view` fail on deprecated `projectCards`; query explicit `--json` fields, edit via REST.
- **Parent PR linkage**: a PR implementing multiple child issues must declare `Fixes #A`, `Fixes #B`, … in the body so all issues close on merge.
- **Commitlint full range**: `npx commitlint --from origin/main --to HEAD`. `scope-enum` is closed — use listed scopes or extend the config in the same PR (as #668 did with `encoder`).
- **Jules force-push risk**: bot can rewrite a PR after your fix, reverting sibling merges. Always `git diff origin/main...HEAD` before merge.
- **Merge order**: independent green PRs first; never `gh pr merge --auto` on stacks (rebase cancellation loop). Single-PR `--auto` with CI green is acceptable.
- **Hygiene**: never commit `export.json` `exported_at`-only hunks or resolver-churn `Cargo.lock` hunks; deny.toml edits are additive-only (deleting ignores re-breaks deny); titles must describe the diff.
- **Mutation in-diff surface**: cosmetic rewrites pull unrelated functions into cargo-mutants — restore-to-main for unrelated lines.
- **`>` vs `>=` top-k**: add a test where `results.len() == top_k` so the `>=` mutant panics.
- **CLI entry-point mutants**: `run_query -> Ok(())` unkillable under `--lib`; exclude in `scripts/mutation_test.sh`.
- **`duplicated_attributes`**: never `#![cfg(test)]` in a file also gated by `#[cfg(test)] mod` in lib.rs.

## Mutation Testing
- **Unreachable code = mutation smell**: audit queue invariants when refactoring guards; remove dead branches.
- **`--in-diff` on post-fix tree**: generate the diff after the fix is staged, not before.
- **Cost**: ~14 min for a 35-line diff (11 mutants) — acceptable for PR validation.
- **New scopes**: add to `commitlint.config.cjs` when creating workspace crates.

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

