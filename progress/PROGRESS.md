## 2026-07-18: Framework ops perf (#524–#526)

### Summary
Single PR implementing all open GitHub issues under GOAP orchestration:
namespace single-clone (#524), parallel `inject_concepts` build (#525),
split import write locks (#526).

### Changes
- `self.namespace().await` once per op (tokio guard cannot cross await)
- Rayon `par_iter` ConceptBuilder construction before `durable_inject_concepts`
- Shared `apply_import_payload` / `clear_for_import_replace` / `persist_import`
- Tests + `bench_inject_concepts_batch` (1/10/100/1000)

### Issues
Fixes #524, #525, #526

---

## 2026-07-18: Open PR Triage (GOAP + Swarm)

### Summary
GOAP-orchestrated sweep of all open PRs with swarm agents for CI repair.
Merge order enforced (no multi-PR auto-merge). State: `plans/GOAP_ORCHESTRATOR.md`,
`plans/GOAP_STATE.md` (`action_last_completed: open_pr_triage_2026_07_18`).

### Merged (correct order)
| PR | Title | SHA | Notes |
|----|-------|-----|-------|
| #528 | BM25 search hot loop | `f8b2bbc` | All-green first; RefMut/slice elision dual-surface |
| #527 | Rayon `probe_batch` / `probe_batch_cached` | `1e94c11` | Fixes #522; CI repair via worktree agent |
| #529 | hybrid `merge_results` partial top-k | `d2db671` | Fixes #523; Jules regression cleaned |

### Closed
| PR | Reason |
|----|--------|
| #520 | Empty Jules research simulation (0 file changes) |

### CI repair patterns (#527 / #529)
1. **commitlint**: invalid scope `ops` → squash to `framework`; full-range check
2. **clippy**: `duplicated_attributes` from `#![cfg(test)]` + `#[cfg(test)] mod`
3. **mutation**: shrink unrelated in-diff surface; kill `>` vs `>=` with len==top_k boundary test; exclude CLI `run_query` under `--lib` mutation
4. **Jules overwrite**: bot force-push can revert sibling merges — reset to main and re-apply minimal delta

### Remaining open issues (not this triage)
#524 namespace reads, #525 inject parallel, #526 import lock hold

---

## 2026-05-23: Documentation Audit

### Summary
Synchronized README, architecture docs, and books with current implementation state, explicitly detailing zero-allocation query caches and WASM limitations while removing outdated fluff.

## 2026-04-11: Persistence Field Integrity Hardening

### Summary
Closed data-integrity gaps that could drop TTL and canonical linkage fields across batch persistence and binary export/import paths, and added workflow-level regression guards.

### Changes
- Updated persistence schema target to v6 and added migration for `canonical_concept_ids_json`.
- Unified single and batch concept saves to persist both `expires_at` and canonical concept IDs.
- Updated concept load paths to deserialize canonical IDs from persistence.
- Extended binary export payload to preserve `expires_at` and `canonical_concept_ids` roundtrip.
- Exposed `expires_at` and `canonical_concept_ids` in WASM concept serialization.
- Relaxed absolute-path validation for export targets where the output file does not yet exist.
- Added targeted regressions:
  - `batch_save_concepts_preserves_ttl_and_canonical_ids`
  - `binary_import_export_preserves_ttl_and_canonical_links`
- Updated CI workflow with explicit persistence field-regression test step.

## 2026-04-11: PR Triage, Issue Planning & Maintenance

### Summary
Triaged open PRs, merged branchless optimization, closed duplicate, labeled new issue, and compacted project learnings.

### PR Activity
- **PR #66** (merged): Branchless bitmask construction in `bundle.rs` and `hyperdim.rs` — ~40% finalize speedup, ~10% bundle speedup. CI all green. 6 lines changed.
- **PR #65** (closed as duplicate): Same optimization as #66 but also attempted to convert `hyperdim.rs` from flat file to directory module (`src/hyperdim/hyperdim_tests.rs`). Unnecessary structural change since file is at 499 LOC (under 500 limit).

### Issue Activity
- **Issue #67** (open, labeled `enhancement`): Ship `csm` CLI binary via npm using napi-rs or separate package. Deferred — requires cross-platform CI build matrix and is low priority vs WASM library usage.

### GOAP State Updates
- `branchless_bundling_optimization: true` — PR #66 merged
- Issue #67 tracked as deferred enhancement (Phase: post-v1.0)

### Current State (v0.3.2)
- ✅ All tests passing (102 tests)
- ✅ All CI checks green
- ✅ No open PRs
- ✅ 1 open issue (#67 - deferred enhancement)
- ✅ Learnings compacted (removed duplicate entries)
- ✅ hyperdim.rs at 499 LOC (at limit, monitor)
- ✅ WASM library: 852KB (under 1MB limit)

## 2026-04-09: Benchmark Harness Optimizations (v0.3.1, v0.3.2)

### Summary
Released v0.3.1 and v0.3.2 with BM25 parallelization, percentile indexing fixes, and new metrics.

### v0.3.2 Changes
- Added p99 latency percentile and NDCG@k scoring
- Floor-based percentile indexing (`(count-1)/2` for median)
- Configurable abstain threshold CLI parameter
- sysinfo API updated to v0.33
- 19 benchmark tests passing

### v0.3.1 Changes
- BM25 parallel scoring with Rayon `par_iter()`
- ~40% BM25 search speedup (3.2ms → 1.9ms for 1000 docs)
- Reservoir step: ~57μs (well under 100μs target)
- Singularity probe 50k: ~3.7ms (excellent scalability)
- Fixed CLI binary shadowing issue

### Technical Insights
- `latencies[count/2]` biased high for even arrays; use `(count-1)/2`
- sysinfo v0.33 changed `refresh_process()` API
- HashSet for gold evidence: O(1) vs O(n) nested iteration

## 2026-04-08: Benchmark Suite Hybrid Retrieval Implementation

### Summary
Implemented hybrid retrieval (BM25 + HDC) with session-scoped queries, achieving 30x improvement in Recall@1.

### Changes
- **MemoryAdapter**: Added BM25 index, hybrid merge with query-length-dependent weights
- **Generator**: Replaced synthetic tokens with semantically meaningful data (real colors, cities)
- **Runner**: Added session-scoped retrieval for Recall/Update/Temporal queries
- **AGENTS.md**: Documented hybrid retrieval strategy and rationale

### Key Bug Fix
HDC returns low-similarity matches (~0.12) for unrelated documents. After min-max normalization,
these become 1.0 and compete with correct BM25 results. Fixed by adding threshold filter (0.15).

### Performance Improvement
| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Recall@1 | 2.5% | 75% | 30x |
| Recall@5 | 10% | 75% | 7.5x |
| MRR | 7.1% | 75% | 10x |

### TaskType Results
- Recall: 100% correct at rank 1
- Temporal: 100% correct at rank 1
- Update: 100% correct at rank 1
- Abstain: Returns results (needs threshold tuning)

### Current State
- ✅ 10 benchmark tests passing
- ✅ Hybrid retrieval working correctly
- ✅ Session-scoped retrieval implemented
- 📝 Abstention threshold needs tuning

### Commits
- `813fb14`: feat(benchmark): hybrid retrieval with session-scoped queries

### Next Steps
1. Tune abstention threshold
2. Add Association and MultiSession test cases
3. Consider lemmatization for better temporal/update query matching

## 2026-04-08: Benchmark Suite Quality Improvements

### Summary
Fixed critical text storage bug and added comprehensive unit tests for benchmark suite (Issue #61).

### Changes
- **Bug Fix**: MemoryAdapter now uses inject_text_with_metadata() to store `_text` metadata
- **Tests Added**: 10 unit tests (memory_adapter: 2, scorer: 8)
- **Documentation**: Added benchmarks/.gitignore for generated results

### TaskType Coverage Analysis
| TaskType | Generated | Status |
|----------|-----------|--------|
| Recall | ✅ | Full coverage |
| Update | ✅ | Partial (no version history) |
| Temporal | ✅ | Minimal (no reservoir) |
| Abstain | ✅ | Full coverage |
| Association | ❌ | **Not generated** |
| MultiSession | ❌ | **Not implemented** |

### Feature Gaps Identified
- Semantic Bridge (ADR-0061): Not tested
- BM25/Hybrid retrieval: Not tested
- TTL (Time-to-Live): Not tested
- Version history: Not tested
- Association graph: Defined but not generated

### Current State
- ✅ 10 new benchmark tests passing
- ✅ Main crate tests passing (25 tests)
- ✅ No build warnings
- ✅ Benchmark runs successfully (Recall@1: 0.025, MRR: 0.073)

### Commits
- `b07be75`: fix(benchmark): text storage bug and add unit tests

### Next Steps
1. Generate Association test cases in generator.rs
2. Add TTL and version history tests
3. Consider BM25/hybrid retrieval mode testing

## 2026-04-08: v0.3.0 Release Preparation

### Summary
Prepared and validated v0.3.0 release with Semantic Bridge Layer, Hybrid Retrieval, and Database Table Prefix.

### Changes Since v0.2.9
- **Semantic Bridge Layer (Issue #52, ADR-0061)**: CanonicalConcept, ConceptGraph, BridgeRetrieval pipeline
- **Hybrid BM25+HDC Retrieval (ADR-0062)**: Query-length-dependent score fusion
- **Database Table Prefix (ADR-0063)**: `csm_` prefix for namespace isolation
- **Schema Migration v5**: Backward-compatible table renaming
- **Encoder Test Refactor**: Moved to tests/encoder_tests.rs
- **WASM Size Gate Fix**: Script now checks library (~870KB) not CLI binary (~5KB)

### Current State
- ✅ 100 tests passing
- ✅ All CI checks passing
- ✅ WASM size gate passed (850.32 KiB < 1MB limit)
- ✅ CHANGELOG updated for v0.3.0
- ✅ ADR-0062, ADR-0063 documented
- ✅ Merged PR #59 (security fix), PR #60 (BM25 optimization)

### Multi-Persona Code Review (Analysis Swarm)
**RYAN findings:**
- Migration v5 properly checks old table existence
- All SQL queries use prefixed tables
- AGENTS.md compliance verified (LOC, parameterized queries)

**FLASH findings:**
- No blockers - CI passing
- Zero manual intervention for migration

**SOCRATES findings:**
- Breaking change documented in ADR-0063
- External SQL tools must update table references

### Technical Insights
- `find | head -n 1` is unreliable for file selection - filesystem order is not deterministic
- WASM library (`chaotic_semantic_memory.wasm`) is ~870KB, CLI binary (`csm.wasm`) is ~5KB
- Schema migration v5 handles both new databases and existing databases with old table names

## 2026-07-23: Wave 33 — CI Fixes + GOAP Orchestrator Hardening

### Summary
Executed Wave 33 plan: fixed all CI failures, resolved BM25 absence TODO, enhanced GOAP orchestrator with swarm patterns.

### PRs Created
- **PR #551**: `fix(ci): use --target nodejs for WASM smoke test and guard main concurrency`
  - Switched CI WASM build from `--target web` to `--target nodejs` (fixes persistent `fetch()` + `file://` failure)
  - Added `module.default` fallback in `wasm/test.js` for CJS module interop
  - Guarded `cancel-in-progress` to only cancel PR runs, not main pushes
- **PR #552**: `fix(retrieval): remove unused is_known_absent function`
  - Removed dead code: zero callers, zero tests, TODO since Wave 32 audit
  - Absence recording infra (persist_absence, AbsenceEntry, AbsenceStore) untouched
- **PR #553**: `feat(goap): enhance orchestrator with swarm patterns and status commands`
  - Added `status` command: current wave, queued/in-progress actions, open PRs
  - Added `wave <N>` command: display wave plan with parallel breakdown
  - Enhanced `verify` command: LOC gate + ADR parity checks
  - Created `references/wave-execution.md`: multi-agent wave execution template
  - Documented swarm dispatch patterns, merge order rules, PR triage integration

### Actions Resolved
- `implement_or_remove_bm25_absence_short_circuit` → resolved (removed)
- CI WASM smoke test persistent failure → fixed
- CI concurrency cancellation rate → fixed
- GOAP orchestrator skill → hardened with swarm patterns

### Key Decisions
- **BM25 absence**: chose "remove" over "wire in" — zero callers, zero tests, complex invalidation semantics
- **WASM target**: dual-target strategy (nodejs for CI smoke, web for release)
- **CI concurrency**: branch-conditional cancel-in-progress prevents cascading failures

## 2026-04-06: Release Workflow Production Solution

### Summary
Diagnosed and documented production-ready npm publish solution using OIDC Trusted Publishing.

### Root Cause
**v0.2.9 npm publish failed** due to duplicate CHANGELOG header at release time:
- Commit `795ad92` had `## [0.2.9]` on line 8 AND `## [0.2.9] - 2026-04-06` on line 9
- This broke awk extraction in release workflow
- Fixed in commits `ee5786e` (remove duplicate) and `4b0f088` (add version link)

