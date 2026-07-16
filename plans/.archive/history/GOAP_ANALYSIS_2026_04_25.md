# GOAP Analysis: Missing Implementation, Docs, New Features
**Date**: 2026-04-25
**Orchestrator**: goap_analysis_2026_04_25
**Baseline**: 333 tests passing, CI queued (PR #111), v0.3.4 current

---

## Current World State Summary

| Metric | Value |
|---|---|
| Crate version | 0.3.4 |
| Total tests | 333 (GOAP_STATE says 284 — **stale**) |
| Source LOC | ~10,778 across 55 .rs files |
| CI status | queued (PR #111 + main push) |
| `cargo doc` warnings | **3 unresolved links** |
| CHANGELOG link refs | **Missing v0.3.2, v0.3.4, v0.1.2, v0.1.3** |
| npm latest published | 0.2.5 (missing 0.2.6–0.3.4) |
| ADRs in docs/adr/ | 2 (most ADRs referenced in GOAP_STATE don't exist in docs/adr/) |

---

## GAP ANALYSIS

### A. Missing Implementation (from GOAP_STATE `false` flags)

| # | Gap | Priority | GOAP Key | Cost |
|---|-----|----------|----------|------|
| A1 | **Inertial reservoir benchmarks** | HIGH | `inertial_reservoir_benchmarked: false` | 3 |
| A2 | **npm publish for v0.2.6–0.3.4** | MED | `npm_v028_published: false`, OTP blocked | 2 |
| A3 | **AVX2/NEON SIMD paths** | LOW (deferred) | `avx2_simd_added: false` | 8 |
| A4 | **Error #[source] attributes** | LOW (deferred) | `error_source_attributes_added: false` | 2 |
| A5 | **Property-based security tests** | LOW (deferred) | `security_tests_added: false` | 3 |
| A6 | **Error remediation hints** | LOW (deferred) | `error_context_enhanced: false` | 2 |

### B. Documentation Gaps

| # | Gap | Priority | Cost |
|---|-----|----------|------|
| B1 | **3 `cargo doc` unresolved links** (`try_remove`, `shortest_path_hops`, `shortest_path`) | HIGH | 1 |
| B2 | **CHANGELOG missing version links** (v0.3.2, v0.3.4, v0.1.2, v0.1.3) | MED | 1 |
| B3 | **GOAP_STATE.md stale test count** (says 284, actual 333) | MED | 1 |
| B4 | **ADR directory sparse** — only 2 of 65+ referenced ADRs exist in docs/adr/ | LOW | 4 |
| B5 | **CHANGELOG has `[Unreleased]` below `[0.3.0]`** — ordering anomaly | LOW | 1 |

### C. New Feature Opportunities

| # | Feature | Priority | Rationale | Cost |
|---|---------|----------|-----------|------|
| C1 | **Semantic Bridge retrieval book chapter** | MED | `bridge_retrieval.rs` (386 LOC) + `semantic_bridge.rs` (400 LOC) have no book chapter | 2 |
| C2 | **SemanticTriples book chapter** | MED | `semantic_triples.rs` exists but has no book coverage | 2 |
| C3 | **TTL/expiry book chapter** | LOW | TTL system (`framework_ttl.rs`, `singularity_ttl.rs`) undocumented in book | 2 |
| C4 | **Batch hyperdim operations** | LOW | `hyperdim_batch.rs` exists, not in book | 1 |
| C5 | **Inertial reservoir book chapter** | MED | `reservoir_inertial.rs` implemented but not in book | 2 |

---

## GOAP ACTION PLAN

### Target State
```yaml
inertial_reservoir_benchmarked: true
cargo_doc_warnings: 0
changelog_links_complete: true
goap_state_test_count_current: true
book_chapters_complete: true
```

### Ordered Actions (A* minimal path, cost-weighted)

```yaml
actions:
  # ─── PHASE 1: DOCS FIX (cost: 3, no code risk) ───
  - name: fix_cargo_doc_unresolved_links
    preconditions:
      core_modules_created: true
    effects:
      cargo_doc_warnings: 0
    cost: 1
    status: complete
    file: src/lib.rs, src/bundle.rs, src/graph_traversal.rs
    description: |
      Fix 3 unresolved doc links:
      1. `try_remove` — likely in bundle.rs BundleAccumulator docs
      2. `shortest_path_hops` — in graph_traversal.rs docs
      3. `shortest_path` — in graph_traversal.rs docs
      Add proper intra-doc links: [`try_remove`](BundleAccumulator::try_remove), etc.
      Completed: 2026-04-30 (verified in GOAP_STATE: cargo_doc_warnings resolved)

  - name: fix_changelog_version_links
    preconditions: []
    effects:
      changelog_links_complete: true
    cost: 1
    status: complete
    file: CHANGELOG.md
    description: |
      Add missing version comparison links at bottom of CHANGELOG.md:
      - [0.3.4]: ...compare/v0.3.2...v0.3.4
      - [0.3.2]: ...compare/v0.3.1...v0.3.2
      - [0.1.3]: ...releases/tag/v0.1.3
      - [0.1.2]: ...releases/tag/v0.1.2
      Fix [unreleased] to compare against v0.3.4 (not v0.3.1).
      Fix [Unreleased] section ordering (currently below [0.3.0]).
      Completed: 2026-04-30 (verified in GOAP_STATE: changelog_links_complete: true)

  - name: update_goap_state_test_count
    preconditions: []
    effects:
      goap_state_test_count_current: true
    cost: 1
    status: complete
    file: plans/GOAP_STATE.md
    description: |
      Update tests_passing: 284 → 333, total_tests: 284 → 333.
      Update orchestrator_last_run and timestamp.
      Completed: 2026-04-30 (GOAP_STATE now shows tests_count: 667)

  # ─── PHASE 2: MISSING IMPLEMENTATION (cost: 3) ───
  - name: benchmark_inertial_reservoir
    preconditions:
      inertial_reservoir_tested: true
    effects:
      inertial_reservoir_benchmarked: true
    cost: 3
    status: complete
    file: benches/benchmark.rs
    description: |
      Add benchmark groups comparing standard vs inertial reservoir:
      1. reservoir_step_beta0 (baseline)
      2. reservoir_step_beta015 (inertial)
      3. reservoir_sequence_10 comparison
      4. Memory retention curve (cosine similarity decay)
      Acceptance: <10% throughput regression at beta=0.15
      Completed: 2026-04-30 (verified: benches/benchmark.rs contains
      inertial_reservoir group with step_50k_beta0, step_50k_beta015 benchmarks)

  # ─── PHASE 3: BOOK CHAPTERS (cost: 6) ───
  - name: add_semantic_bridge_book_chapter
    preconditions:
      core_modules_created: true
    effects:
      book_semantic_bridge_chapter: true
    cost: 2
    status: complete
    file: book/src/semantic-bridge.md, book/src/SUMMARY.md
    description: |
      Add book chapter covering SemanticBridge and BridgeRetrieval APIs,
      hybrid retrieval patterns, and BM25+vector search integration.
      Completed: 2026-04-30 (verified: book/src/semantic-bridge.md exists,
      listed in SUMMARY.md; GOAP_STATE: book_chapters_complete: true)

  - name: add_inertial_reservoir_book_chapter
    preconditions:
      inertial_reservoir_implemented: true
    effects:
      book_inertial_reservoir_chapter: true
    cost: 2
    status: complete
    file: book/src/inertial-reservoir.md, book/src/SUMMARY.md
    description: |
      Add book chapter explaining inertial ESN dynamics, beta parameter
      tuning, memory retention benefits, and configuration.
      Completed: 2026-04-30 (verified: book/src/inertial-reservoir.md exists,
      listed in SUMMARY.md; GOAP_STATE: book_chapters_complete: true)

  - name: add_ttl_expiry_book_chapter
    preconditions:
      concept_ttl: true
    effects:
      book_ttl_chapter: true
    cost: 2
    status: complete
    file: book/src/ttl.md, book/src/SUMMARY.md
    description: |
      Add book chapter covering concept TTL/expiry system,
      framework_ttl.rs and singularity_ttl.rs APIs.
      Completed: 2026-04-30 (verified: book/src/ttl.md exists,
      listed in SUMMARY.md; GOAP_STATE: book_chapters_complete: true)
```

### Dependency Graph

```
fix_cargo_doc_unresolved_links ──┐
fix_changelog_version_links ─────┤──→ (no deps, parallel)
update_goap_state_test_count ────┘
                                       │
benchmark_inertial_reservoir ──────────→│ (depends on tested=true, already met)
                                       │
add_semantic_bridge_book_chapter ──────→│
add_inertial_reservoir_book_chapter ───→│ (parallel, no interdeps)
add_ttl_expiry_book_chapter ───────────→│
```

### Deferred (not in plan, requires user demand)
- AVX2/NEON SIMD (cost: 8, needs arch-specific testing)
- Error #[source] attributes (cost: 2, API-breaking)
- Property-based security tests (cost: 3)
- npm publish v0.2.6+ (blocked by OTP requirement)
- ADR directory backfill (cost: 4, 63+ missing ADR files)
