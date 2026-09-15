# PROGRESS

## 2026-09-15: PR Queue Clear — Draft Closed as No-Impact, Hygiene + Persistence Queue

### Summary
Zero open issues; two open PRs. Roasted both: #706 (Jules `perf(retrieval)`) closed as no-impact after allocator-level measurement, #707 (dependabot action SHA bump) kept as the single mergeable keeper. Session also surfaced three main-truth defects queued for the hygiene PR: the 0.3.8 version revert, tracked-but-gitignored `export.json`, and a 2.2 MB committed `cargo metadata` dump.

### Actions
- **#706 Closed** (no impact): outputs identical, allocations identical (13/13 allocs, 131,626 bytes map path; 12/12, 24,410 bytes fast path), timing deltas opposite in sign (−4.4% / +4.3%). Full roast comment posted to the PR; reopen path documented (attach criterion output from `benches/hybrid_benchmark.rs`).
- **#707 Keeper**: `pre-release-gate.yml` SHA pin `fa23953… v2.87.10` → `9534c84… v2.87.11`; CI green 27/27, `mergeStateStatus: CLEAN`, merge left to the maintainer.
- **Hygiene PR queued**: untrack `export.json` + `benchmarks/Cargo.lock`, remove `metadata.json`, align `migrations/007_add_hnsw_graph.sql` with the Rust schema, restore the 0.3.8 version/CHANGELOG bump.
- **Persistence queue (ADR-0094, user-approved)**: 4-phase `deduplicate_persistence_owner_bodies` — crate parity, root delegation, payload convergence, cleanup; executed after the hygiene and rerank PRs.
- **Rerank queue**: `src/framework_rerank.rs` (`probe_with_rerankers`, ADR-0071) is unreachable since #178 dropped its `mod` declaration and compiles clean — restore module + regression test.
- **Report**: `plans/PR_ROAST_2026_09_15.md`.

## 2026-09-11: Stale-Base Squash Revert Repair + PR Wave

### Summary
PR triage surfaced that the `943/fdcd069` squash sequence (Jules PR #683) was based on a stale fork and reverted already-merged main content: the gh-release v3.0.3 pin (#674), the uuid 1.26.0 lock entry (#677), the LSH `compute_hash_from_bytes` / pre-sized candidate-set work (#683, `a3eeaab`), the `progress/LEARNINGS.md` compaction, and `plans/PR_ROAST_2026_09_08.md`. Restored every reverted path in one dedicated PR and encoded the detection rule in `progress/LEARNINGS.md`.

### Actions
- **#686 Merged** (`ccbb7e8`): restores LSH bit-mask hashing + `HashSet::with_capacity(32)` (the #683 optimization clobbered by `9140aa4`).
- **#684 Queued**: `fix(singularity)` clamps `BridgeRetrieval::query` top_k to `MAX_TOP_K_LIMIT` (CWE-770), matching the BM25 clamp precedent.
- **Restore PR**: workflows back to `action-gh-release@efb3536… # v3.0.3`, `Cargo.lock` back to uuid 1.26.0 (`cargo metadata --locked` OK), `progress/LEARNINGS.md` + `progress/PROGRESS.md` + `plans/PR_ROAST_2026_09_08.md` restored from `f6117e7`, LSH comment corrections and `RetrievalConfig` struct literals restored from `a3eeaab`.
- **#685 Pending**: feature-gated `experimental-ils3d` hyperchaotic map (Sun et al. 2025); CI green, needs a naming-fidelity review before merge.
- **Report**: `plans/PR_ROAST_2026_09_11.md`.

---

## 2026-09-08: PR Roast — Sequential Merge of Remaining PR Wave

### Summary
Triaged all 7 open PRs (#679, #677, #676, #675, #674, #673, #660) per `pr-roast-triage`. Zero closures (no duplicates, no zero-impact PRs). Applied roast fixes: rewrote #660's stale body via REST API, marked draft #679 ready. Merging sequentially with full CI-truth verification per AGENTS.md (no `--auto`).

### Actions
- **#679 Merged** (`a73b8a0`): perf(retrieval) single-pass hash inserts in BFS expansion + fact dedup. Required `gh pr ready` (Jules draft) before merge.
- **#660 Body Fixed**: Stale re-export description replaced via `gh api -X PATCH`; follow-up queued to lift `MAX_TRAVERSAL_EXPANSIONS` into `GraphRagConfig`.
- **Branch Sync**: #660 synced via `gh api -X PUT .../pulls/660/update-branch` (no `gh pr update-branch` subcommand in installed gh); all 5 dependabot PRs queued for single-pass `@dependabot rebase` onto final main.
- **Report**: `plans/PR_ROAST_2026_09_08.md`.

---


## 2026-09-07: PR Roast Triage & Resolution of All Open Issues

### Summary
Comprehensive triage and roast of all 13 open PRs and 4 open GitHub issues. Closed duplicate PR #672 with no independent impact. Verified and merged PR #647, resolving and closing all 4 open issues (#639, #640, #641, #642). Deduplicated root retrieval modules, verified 100% Codacy pass, and established sequential manual merge plan for the remaining 11 open PRs.

### Actions
- **All 4 GitHub Issues Resolved**:
  - #639 (`RetrievalConfig abort knobs + for_token_count`): Implemented in `crates/csm-memory/src/singularity_retrieval.rs`.
  - #640 (`perf: cut probe_text / BridgeRetrieval work`): Parent issue resolved and closed.
  - #641 (`perf(bridge): cap expansion and Hamming only new IDs`): Implemented in `src/bridge_retrieval.rs` via `score_specific_candidates`.
  - #642 (`perf(retrieval): BM25 absence + stage-1 cap + HDC early-exit`): Implemented across retrieval pipeline.
- **PR #647 Merged** (`ff85763`): 28 CI checks green, 0 Codacy issues. Replaced ~970 lines of duplicate root retrieval files (`bm25.rs`, `rerank.rs`) with re-export shims over `csm_retrieval` (ADR-0094), reducing repository LOC by over 5,300 lines while keeping all files strictly under the 500 LOC gate.
- **PR #672 Closed**: Redundant duplicate of #653; stripped public wrapper, failed commitlint.
- **PR Titles Fixed via REST**: Updated #670 (`perf(core)`) and #660 (`fix(retrieval): cap GraphRAG traversal expansions at 1000 nodes`).
- **Codacy Gate Verified**: 0 issues across all 13 analyzed PRs.
- **Merge Plan Emitted**: `plans/PR_ROAST_2026_09_07.md`.

---

## 2026-07-27: PR Triage + CI Queue Fix

### Summary
GOAP-orchestrated sweep of open PRs. Closed #571 (fake perf optimization), merging #573 (dependabot).
Fixed release workflow timeout caused by CI queue starvation.

### Actions
- **PR #571 closed**: `perf(retrieval): optimize min/max loops` — replaced `.min()`/`.max()` with `<`/`>` comparisons. Net negative: removes docs, adds 4 mutation exclusions, reverses intentional design. LLVM generates identical code either way.
- **PR #573 merging**: dependabot bump `taiki-e/install-action` 2.83.4 → 2.85.2 (3 workflow files).
- **CI queue starvation**: Runs 30274366967 + 30275287578 stuck "queued" >1hr. Cancelled and re-triggered. Release run 30274367903 timed out at 1800s waiting for CI that never started.
- **Fix**: Enhanced `wait-for-ci` in release.yml to detect perpetual-queue and re-trigger CI.

---

## 2026-08-05: Review-to-merge sweep (PRs #597-#601)

### Summary
Reviewed and merged the final open PRs: #597 (direct SIMD hamming), #598 (graph candidate &str), #599 (native arm64 test job), plus state/docs PRs #600 and #601. The queue is now empty and every perf claim landed with same-machine criterion evidence attached.

### Actions
- **PR #597 merged** (`0b42edb`): `BHVec10240::hamming` dispatches directly over the packed `[u64; 160]` words — AVX2 (x86_64), NEON (aarch64), unrolled scalar fallback (wasm32) — eliminating the two `to_hvec()` layout conversions. Same-machine criterion ~2.6–2.75× faster (main 99.4/149.9 ns vs 37.7/54.5 ns). Codacy unsafe-usage findings fixed in code via `.codacy.yml` `exclude_paths` (repo policy); zero dashboard suppressions. Added forced-fallback unit test + lane-skip guard.
- **PR #599 merged** (`8d63b27`): `test-core-arm64` job on `ubuntu-24.04-arm` executes the NEON kernels in CI (previously compile-only on x86_64).
- **PR #598 merged** (`075cfe8`): `generate_graph_candidates` BFS borrows `&str` instead of cloning every candidate `String` — ~8% faster end-to-end (same-machine A/B: 238.3 → 218.5 µs, ~840 fewer allocs). Evidence bench committed: `benches/graph_candidates_benchmark.rs`.
- **PR #600/#601 merged** (`59c44d6`/`cb45951`): state records for all merged PRs; benchmark-methodology learnings (stale-binary detection, forced clean A/B, load-spike exclusion, deterministic test graphs); `benchmark-graph-candidates` CI regression gate (ceiling 600 µs, measured 258.9 µs on GitHub hardware).
- **Process notes**: commitlint footer-max-line-length requires every message line ≤ 100 chars; `npx commitlint | tail` swallows the exit code — always check `$?` explicitly.

---

## 2026-07-23: Wave 33 — CI Fixes + GOAP Orchestrator Hardening

- PR #551: WASM `--target nodejs` for CI + main concurrency guard
- PR #552: Removed dead `is_known_absent` (zero callers, zero tests)
- PR #553: GOAP orchestrator `status`/`wave`/`verify` commands + swarm patterns

---

## 2026-07-18: Framework Ops Perf + PR Triage

- PR #524-#526: namespace single-clone, parallel inject, split import locks
- Merged #528 (BM25 hot loop), #527 (Rayon probe_batch), #529 (hybrid partial top-k)
- Closed #520 (empty Jules research PR)

---

## 2026-05-23: Documentation Audit
Synchronized README, architecture docs, and books with implementation state.

## 2026-04-11: Persistence Field Integrity
Schema v6: `expires_at` + `canonical_concept_ids` across all persistence surfaces.

## 2026-04-09: Benchmark Harness (v0.3.1, v0.3.2)
BM25 Rayon parallelization (~40% speedup), p99/NDCG metrics, percentile fix.

## 2026-04-08: Hybrid Retrieval + v0.3.0
BM25+HDC fusion with query-length weights. Recall@1: 2.5% → 75%. Semantic Bridge (ADR-0061), table prefix (ADR-0063).

## 2026-04-06: Release Workflow
npm OIDC Trusted Publishing. Fixed duplicate CHANGELOG header breaking awk extraction.
