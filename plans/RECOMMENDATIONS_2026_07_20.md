# Codebase Recommendations — 2026-07-20

**Branch audited:** `feat/framework-ops-perf-524-525-526` (+ open PR #532, #534)  
**Product version:** `0.3.7`  
**Prior roadmap:** Wave 32 ([`GOAP_AUDIT_2026_07_14.md`](GOAP_AUDIT_2026_07_14.md), ADR-0093–0096)  
**Plans compaction:** [`ARCHIVE_MANIFEST.md`](ARCHIVE_MANIFEST.md)

This document consolidates improvements, missing implementations, optimizations, new features, and documentation/skill hygiene. It is the active backlog companion to `ACTIONS.md`.

> **2026-08-08 (ADR-0097):** statuses reconciled — the authoritative queue is
> now the active-only `ACTIONS.md` (6 queued actions). Items this file listed
> as queued that have since landed (BM25 absence removal, TTL lifecycle
> ownership, retrieval ownership consolidation, CLI metrics reset, f32 API
> hardening) are complete; see ADR-0097 and the archive snapshot.

---

## Executive summary

| Area | Health | Headline |
|------|--------|----------|
| Core library | Strong | Green LOC gate, rich Framework/MCP/WASM/CLI surface |
| Correctness (Wave 32 P0) | Mostly landed | ANN revision, persistence authority, fuzz build, skill fail-closed |
| Ownership / crates | **Weak** | Root `src/` vs `crates/*` still dual-write; retrieval/persistence **diverged** |
| Evidence / claims | **Weak** | Formula-only 10M memory test; incomplete scale benches; mutation exclusions |
| Docs (README/AGENTS) | Medium | Version drift, ANN “deferred” vs shipped backends, skill count wrong |
| Skills | Medium | 32 skills; 3 approach LOC ceiling; catalog not single-sourced |
| Plans | **Improved** | Active set compacted; 74 historical files archived 2026-07-20 |

**Do next (priority order):**

1. **Finish open PR merge queue** — #532 (framework ops #524–#526), #534 (hybrid min-max).  
2. **Wave 32 P1 ownership** — feature contracts → retrieval façade → persistence/CLI/WASM.  
3. **Close missing behavior** — BM25 absence TODO, TTL task ownership, CLI `metrics reset`.  
4. **Evidence tiers** — measured memory model, ANN/persistence scale, fuzz short/scheduled runs.  
5. **Docs/skills pass** — README truth, AGENTS skill inventory, skill LOC & catalog.

---

## 1. Missing implementations

### 1.1 Production TODO / incomplete APIs

| ID | Item | Evidence | Recommendation | Priority |
|----|------|----------|----------------|----------|
| M1 | BM25 absence short-circuit | `src/retrieval/bm25.rs` `is_known_absent` + `// TODO: Wire into…` | Wire into hybrid path with threshold/invalidation tests **or** remove unused API | P1 |
| M2 | CLI `metrics reset` | `src/cli/commands/metrics.rs` returns “not yet implemented” | Implement counter zeroing (framework metrics) or remove subcommand from help | P2 |
| M3 | TTL background cleanup lifecycle | `tokio::spawn` without owned `JoinHandle`/cancel (audit F2) | Own cancel token + bounded shutdown on drop | P1 |
| M4 | Persistence-disabled false success | `src/lib.rs` / builder no-ops when features off | `UnsupportedOperation` or cfg-absent APIs (ADR-0094) | P1 |
| M5 | Lean `--no-default-features` | Root default still pulls persistence/parallel paths inconsistently | Forward features to owner crates; cargo-tree gate | P1 |

### 1.2 Wave 32 actions still queued (authoritative list)

From `plans/ACTIONS.md` (status `queued` as of this audit):

| Priority | Action | Cost | Theme |
|----------|--------|------|-------|
| P1 | `enforce_workspace_feature_contracts` | 5 | Features |
| P1 | `replace_persistence_disabled_noops` | 3 | API honesty |
| P1 | `align_wasm_ci_release_artifact` | 3 | One WASM artifact |
| P1 | `consolidate_retrieval_ownership` | 8 | Dual source of truth |
| P1 | `consolidate_persistence_cli_wasm_ownership` | 10 | Dual source of truth |
| P1 | `own_ttl_cleanup_lifecycle` | 3 | Lifecycle |
| P2 | `implement_or_remove_bm25_absence_short_circuit` | 3 | TODO |
| P2 | `establish_tiered_benchmark_evidence` | 8 | Evidence |
| P2 | `add_ann_and_persistence_scale_benchmarks` | 8 | Evidence |
| P2 | `replace_formula_only_memory_claim` | 4 | Evidence |
| P2 | `harden_mutation_evidence` | 4 | Mutation honesty |
| P2 | `fuzz_short_and_scheduled_runs` | 4 | Fuzz runtime |
| P2 | `run_critical_skill_behavioral_evals` | 5 | Skills |
| P3 | `deduplicate_test_and_source_surfaces` | 8 | Drift |
| P3 | `canonicalize_hooks_skill_refs_and_catalog` | 5 | Skills/hooks |
| P3 | `reconcile_harness_engineering_state` | 2 | HARNESS truth |
| P3 | `compact_active_plans_non_destructively` | 3 | **Done 2026-07-20** (this session) |

### 1.3 Ownership divergence (measured)

| Pair | Status | Notes |
|------|--------|-------|
| `src/cli/args.rs` ↔ `crates/csm-cli` | Identical | Safe re-export candidate |
| `src/wasm.rs` ↔ `crates/csm-wasm` | Identical | Safe re-export candidate |
| `src/retrieval/bm25.rs` ↔ `csm-retrieval` | **Diverged** (~82 diff lines) | Perf PRs may only hit one side |
| `src/retrieval/hybrid.rs` ↔ `csm-retrieval` | **Diverged** (~88 lines) | Same risk |
| `src/retrieval/rerank.rs` ↔ `csm-retrieval` | **Diverged** (~20 lines) | Same risk |
| `src/persistence.rs` ↔ `csm-persistence` | **Diverged** (~164 lines) | Highest drift risk |

**Rule:** Never “blind re-export.” Parity tests → pick owner → façade root.

---

## 2. Optimizations

### 2.1 In flight (merge / land)

| Item | Status | Notes |
|------|--------|-------|
| Issues #524–#526 framework ops | Branch + PR **#532** | Namespace clone once; parallel inject construction; split import write holds |
| Hybrid float min-max | Open PR **#534** | Jules-style micro-opt; verify vs mutation/clippy before merge |
| BM25 sparse + hot loop | Merged (#444, #528) | Keep root/`csm-retrieval` in sync when consolidating |
| `probe_batch` Rayon | Merged (#527) | |
| Hybrid partial top-k merge | Merged (#529) | |

### 2.2 Recommended next perf work

| ID | Optimization | Trigger / rationale | Priority |
|----|--------------|---------------------|----------|
| O1 | Finish ownership so **one** BM25/hybrid body is optimized | Dual edits waste PR work | P0 (process) |
| O2 | ANN scale bench: brute / bucket / HNSW / LSH at 50k–200k | README still frames ANN as “deferred”; code has backends | P1 |
| O3 | Persistence contention p50/p95 with bounded retries | Audit E5 | P2 |
| O4 | Zero-copy / arena for large batch inject after #525 | Measure first | P3 |
| O5 | Quantized / PQ hypervectors (ADR-0075 / goals) | Only when measured RAM pressure | Deferred |
| O6 | Connection pool re-evaluation | Only after benchmark vs ADR-0005/0014 | Deferred |

### 2.3 Do **not** optimize yet

- Configurable hypervector dimensions (ADR-0060) without user demand.  
- Hardware microbench gates on unpinned GHA runners.  
- Claiming 10M concepts / 12MB until measured (see E3).

---

## 3. New features (product)

Prioritize only after Wave 32 ownership/evidence gates.

| ID | Feature | Value | Complexity | Priority |
|----|---------|-------|------------|----------|
| F1 | Absence-aware hybrid retrieval (complete M1) | Fewer wasted BM25 scans on known-miss queries | Low–Med | P1 |
| F2 | CLI `metrics reset` + optional Prometheus scrape file | Ops DX | Low | P2 |
| F3 | Explicit framework `shutdown()` (TTL + pool + tasks) | Clean process exit / tests | Med | P1 |
| F4 | Namespace-scoped export/import CLI filters | Multi-tenant ops | Med | P2 |
| F5 | GraphRAG query presets in CLI (`probe-graph` depth/fanout profiles) | Dogfood GraphRAG | Low | P3 |
| F6 | MCP tool: batch inject + hybrid probe with abstention | Agent ecosystems | Med | P2 |
| F7 | Soft-delete + tombstone TTL | Safer delete semantics | Med | Deferred |
| F8 | Streaming index rebuild / background reindex | Large DBs | High | Deferred |
| F9 | Reader-lite remote embedding cache policies | Cost control | Med | Deferred |
| F10 | DuckDB analytics recipes in book + CLI examples | Companion crate adoption | Low | P3 |

---

## 4. README.md recommendations

| ID | Issue | Fix |
|----|-------|-----|
| R1 | Install examples mix `0.3` and `0.4` while crate is **0.3.7** | Align all examples to `0.3` / `"0.3.7"` until a real 0.4 release |
| R2 | “ANN/LSH Deferred” section contradicts shipped `IndexBackend` (HNSW/LSH/bucket) | Rewrite: default exact scan; optional backends; scale triggers from ADR-0056 |
| R3 | LOC policy says `src/` only | State workspace rule: all `src/` + `crates/**/*.rs` ≤500 |
| R4 | Features list omits MCP, GraphRAG, hybrid BM25, embedding bridge, OTLP | Add short “Optional surfaces” table with feature flags |
| R5 | Real-usage validation uses bare `csm` | Prefer `./target/debug/csm` (AGENTS rule) so installs are not stale |
| R6 | Quick Links lack book / ADR / HARNESS | Link `book/`, `plans/adr/`, `HARNESS.md` |
| R7 | Concurrency claims (“I/O after write lock”) | Re-verify after Wave 32 lock work; keep claims evidence-linked |
| R8 | Performance gates section | Point to `benchmarks/` evidence tiers once ADR-0095 tiers land |

**Suggested README structure tweak (minimal):** keep HDC warning + install; add “Architecture at a glance” (workspace crates); fix ANN section; fix versions.

---

## 5. AGENTS.md recommendations

| ID | Issue | Fix |
|----|-------|-----|
| A1 | “Skills (30 Total)” but **32** `SKILL.md` files | Update count + table to include `goap-orchestrator`, `jules-orchestration` (and any missing) |
| A2 | Key files list is root-`src` centric | Note workspace owners: `csm-core`, `csm-memory`, `csm-retrieval`, `csm-persistence`, `csm-cli`, `csm-wasm` |
| A3 | Phase 1 still says “all source files ≤500” via `src crates` — good; ensure checklist matches CI script | Cross-link `tests/arch_fitness.rs` if present |
| A4 | No pointer to compacted plans | Add: start at `plans/README.md` + `RECOMMENDATIONS_2026_07_20.md` |
| A5 | Release section is solid | Keep; ensure skill `release-management` stays ≤250 LOC (already split) |

---

## 6. `.agents/skills/` recommendations

### 6.1 Inventory (32)

Core workflow, swarm, TRIZ, release, verification, orchestration — generally healthy.

### 6.2 LOC / structure

| Skill | LOC | Action |
|-------|-----|--------|
| `npm-trusted-publishers` | 241 | Near ceiling; extract troubleshooting tables → `references/` |
| `skill-evaluator` | 214 | OK; keep eval fixtures outside SKILL.md |
| `shell-script-quality` | 190 | OK |
| `release-management` | 161 | Good post-split pattern — use as template |
| Rest | ≤165 | Maintain ≤250 hard cap |

### 6.3 Skill program gaps

| ID | Recommendation | Priority |
|----|----------------|----------|
| S1 | `canonicalize_hooks_skill_refs_and_catalog` — single generated catalog from disk | P2 |
| S2 | `run_critical_skill_behavioral_evals` — ≥19/20 on critical five | P2 |
| S3 | Ensure every skill has valid frontmatter (`name`, `description`) enforced by `validate-skill-format.sh` | Done / keep fail-closed |
| S4 | Add thin skill or reference for **workspace ownership map** (which crate owns what) | P3 |
| S5 | `git-workflow` / `github-ci-guardrails` already updated for multi-PR pitfalls — keep as required reading | — |
| S6 | Avoid new skills until catalog + evals land; prefer `references/` expansion | Process |

---

## 7. Tests, CI, quality

| ID | Recommendation | Priority |
|----|----------------|----------|
| Q1 | Fuzz: short PR runs + scheduled full runs (compile-only already) | P2 |
| Q2 | Mutation: timeouts ≠ killed; no silent exclude of changed files | P2 |
| Q3 | After ownership consolidate: unique test owners (stop double-counting) | P3 |
| Q4 | Close issues #524–#526 after #532 merge | P0 |
| Q5 | Pre-release gate: `ci_pre_release_gate_failing` was true historically — re-check before any release | P1 |
| Q6 | Dependabot: 5 alerts blocked upstream (documented deny.toml) — revisit quarterly | P3 |

---

## 8. Plans / process (executed + remaining)

### 8.1 Done this session

- Archived **25** completed GOAP/analysis docs + **49** handoffs → `plans/.archive/2026-07-20-historical/`.  
- Added `plans/README.md`, `ARCHIVE_MANIFEST.md`, `handoffs/README.md` redirect.  
- This recommendations file.

### 8.2 Remaining process

| ID | Item |
|----|------|
| P1 | Mark `compact_active_plans_non_destructively` complete in ACTIONS + GOAP_STATE |
| P2 | Optionally truncate `ACTIONS.md` completed prefix into archive (keep queued tail + last N complete) — separate PR; reference audit first |
| P3 | Snapshot `GOAP_STATE` historical wave blocks into dated archive YAML when Wave 32 closes |

---

## 9. Proposed Wave 33 (after Wave 32 ownership slice)

**Name:** Docs truth + missing behavior + evidence  
**Focus:**

1. Merge #532 / #534; close #524–#526.  
2. `enforce_workspace_feature_contracts` + `replace_persistence_disabled_noops`.  
3. `consolidate_retrieval_ownership` (parity tests first).  
4. `implement_or_remove_bm25_absence_short_circuit` + `own_ttl_cleanup_lifecycle` + CLI metrics reset.  
5. README + AGENTS accuracy pass.  
6. Tiered benchmark evidence + measured memory model.

**Non-goals:** product quantization, new embedding providers, destructive plan deletion.

---

## 10. TRIZ contradictions (session)

| Contradiction | Principle | Application |
|---------------|-----------|-------------|
| Dual crates for modularity vs single truth | Extraction + intermediary | Owner crate + root façade |
| Fast agent context vs audit history | Nested doll | Compact active plans + immutable archive |
| README simplicity vs full feature surface | Taking out | Short tables + deep links to book/ADR |
| Perf PRs vs dual source trees | Prior action | Consolidate before more micro-opts |

---

## 11. Acceptance checklist for “recommendations applied”

- [ ] All §1.2 queued actions have owners / PRs or explicit deferrals  
- [ ] No production `TODO` in retrieval hot path  
- [ ] README version + ANN sections match code  
- [ ] AGENTS skill count = disk inventory  
- [ ] `plans/README.md` is the only plans entrypoint agents need  
- [ ] Archive manifest remains accurate  

---

## 12. References

- [`GOAP_STATE.md`](GOAP_STATE.md)  
- [`ACTIONS.md`](ACTIONS.md)  
- [`GOAP_AUDIT_2026_07_14.md`](GOAP_AUDIT_2026_07_14.md)  
- ADR-0093 authoritative persistence · ADR-0094 workspace contracts · ADR-0095 evidence · ADR-0096 agent validation  
- Issues: #524, #525, #526 · PRs: #532, #534  
