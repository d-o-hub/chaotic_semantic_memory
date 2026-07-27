# PROGRESS

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
