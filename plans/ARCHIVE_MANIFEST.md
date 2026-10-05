# Plans Archive Manifest

**Archive dates:** 2026-07-20, 2026-08-08  
**Policy:** Non-destructive (ADR-0096; extended by ADR-0097 for state-file snapshots)  
**Locations:** `plans/.archive/` (superseded ADR copies), `plans/.archive/2026-07-20-historical/`, `plans/.archive/2026-08-08-historical/`

Historical completed analysis, wave handoffs, one-shot GOAP plans, and
pre-compaction state-file snapshots were moved out of the active `plans/`
root so agents load a compact current state. Nothing was deleted.

## How this file is kept honest

This manifest is the complete index of `plans/.archive/`: every file on disk
under that directory has exactly one row below (131 rows), named by its
repo-relative path in backticks. `scripts/check-archive-manifest.sh` — wired
into `./scripts/validate.sh` as `==> Plan archive manifest completeness` —
fails when a file exists on disk without a row here, or when a row names a file
that is no longer on disk. Before 2026-10-05 nothing read this file, so 55
archived ADRs and 49 handoffs went unlisted while
`plans/GOAP_STATE.md: plan_archive_manifest_valid` reported `true` on prose
alone (issue #831).

Row convention: the checker matches a backticked repo-relative path under
`plans/.archive/` whose last segment looks like a file, anywhere in this file,
so section layout and the description column stay free-form and an archived
file of any extension is listable. Directory paths such as `plans/.archive/`
and placeholders such as `<name>` are not treated as rows.

## Superseded ADR copies in `plans/.archive/` (55 files)

`scripts/plans-manager.sh archive adr` moves ADR files straight into the
`plans/.archive/` root, keeping only the newest ten under `plans/adr/`. These
copies predate later ADR renumbering and slug renames: every number below also
exists as a live ADR in `plans/adr/`, so `plans/adr/` plus
`plans/ADR_REGISTRY.md` stay the source of truth and this table is the
historical record of what was superseded. Titles are each file's own H1.

| File | Title (from its H1) |
|------|---------------------|
| `plans/.archive/0001-use-libsql.md` | ADR-0001: Use libSQL for Persistence |
| `plans/.archive/0002-hypervector-size.md` | ADR-0002: Hypervector Size: 10240 bits |
| `plans/.archive/0004-sparse-reservoir-matrix.md` | ADR-0004: Sparse Reservoir Weight Matrix |
| `plans/.archive/0005-persistence-connection-model.md` | ADR-0005: Persistence Connection Model |
| `plans/.archive/0006-persistence-batch-operations.md` | ADR-0006: Persistence Batch Operations |
| `plans/.archive/0007-similarity-search-optimization.md` | ADR-0007: Similarity Search Optimization |
| `plans/.archive/0008-wasm-rayon-gating.md` | ADR-0008: WASM Rayon Gating Strategy |
| `plans/.archive/0009-partial-reservoir-updates.md` | ADR-0009: Partitioned Reservoir Step Updates For Latency Gate |
| `plans/.archive/0010-public-api-result-contract.md` | ADR-0010: Public API Result Contract |
| `plans/.archive/0011-sqlite-foreign-keys-and-builder-migration.md` | ADR-0011: SQLite Foreign Keys and libsql Builder Migration |
| `plans/.archive/0012-conceptbuilder-metadata-error-propagation.md` | ADR-0012: ConceptBuilder Metadata Error Propagation |
| `plans/.archive/0013-simd-hypervector-operations.md` | ADR-0013: SIMD-Accelerated Hypervector Operations |
| `plans/.archive/0014-connection-pooling-turso.md` | ADR-0014: Connection Pooling for Remote Turso Databases |
| `plans/.archive/0015-structured-logging.md` | ADR-0015: Structured Logging with Tracing |
| `plans/.archive/0016-export-import-migration.md` | ADR-0016: Data Export/Import for Migration |
| `plans/.archive/0017-concept-versioning.md` | ADR-0017: Concept Version History |
| `plans/.archive/0018-input-validation-policy.md` | ADR-0018: Input Validation Policy for Public APIs |
| `plans/.archive/0019-backup-restore-safety.md` | ADR-0019: Backup/Restore Safety with Live SQLite Handles |
| `plans/.archive/0020-silent-data-loss-on-load.md` | ADR-0020: Fix Silent Data Loss on Association Load/Import |
| `plans/.archive/0021-auto-schema-migration.md` | ADR-0021: Automatic Schema Migration on Startup |
| `plans/.archive/0022-wasm-api-parity.md` | ADR-0022: WASM API Parity and Persistence Stub Completeness |
| `plans/.archive/0023-zero-alloc-query-cache.md` | ADR-0023: Zero-Alloc Query Cache Keys and Cached Results |
| `plans/.archive/0024-concept-expiration-ttl.md` | ADR-0024: Concept Expiration (TTL) |
| `plans/.archive/0024-performance-optimizations-phase2.md` | ADR-0024: Performance Optimizations Phase 2 |
| `plans/.archive/0025-weighted-forgetting-decay.md` | ADR-0025: Weighted Forgetting (Association Decay) |
| `plans/.archive/0026-namespace-isolation.md` | ADR-0026: Namespace Isolation (Soft Multi-Tenancy) |
| `plans/.archive/0027-documentation-standards.md` | ADR-0027: Documentation Standards for Public API |
| `plans/.archive/0028-observability-completion.md` | ADR-0028: Observability Completion - Tracing and Metrics |
| `plans/.archive/0029-wasm-api-parity.md` | ADR-0029: WASM API Parity - Exposing Missing Methods |
| `plans/.archive/0030-test-benchmark-gap-remediation.md` | ADR-0030: Test and Benchmark Gap Remediation |
| `plans/.archive/0031-two-tier-architecture-documentation.md` | ADR-0031: Two-Tier Architecture Documentation (YAML + DrawIO) |
| `plans/.archive/0032-cli-robustness.md` | ADR-0032: CLI Robustness: JSON Escaping, Exit Codes, and Error Output |
| `plans/.archive/0033-wasm-panic-safety.md` | ADR-0033: WASM Panic Safety: Replace Unwrap with Error Propagation |
| `plans/.archive/0034-framework-metadata-injection.md` | ADR-0034: Framework Metadata Injection and WASM Batch API Parity |
| `plans/.archive/0035-cache-memory-guardrails.md` | ADR-0035: Cache Memory Guardrails for Similarity Query Cache |
| `plans/.archive/0036-ci-dx-hardening.md` | ADR-0036: CI/DX Hardening: LOC Gate, Pre-Commit Hook, Clippy Parity |
| `plans/.archive/0037-rust-best-practices.md` | ADR-0037: Rust Best Practices: #[must_use], Unsafe Docs, JSON Safety |
| `plans/.archive/0038-cargo-toml-modernization.md` | ADR-0038: Cargo.toml Modernization for crates.io Publishing |
| `plans/.archive/0039-release-engineering.md` | ADR-0039: Release Engineering Strategy |
| `plans/.archive/0040-async-lock-safety.md` | ADR-0040: Async Lock Safety: Avoid Holding RwLock Across Await Points |
| `plans/.archive/0041-batch-similarity-optimization.md` | ADR-0041: Batch Cosine Similarity Performance Optimization |
| `plans/.archive/0043-skill-memory-security-hardening.md` | ADR-0043: Skill-Memory Security Hardening |
| `plans/.archive/0044-memory-limits-governance.md` | ADR-0044: Memory Limits and Resource Governance |
| `plans/.archive/0045-security-input-validation.md` | ADR-0045: Security Policy for Input Validation |
| `plans/.archive/0047-security-performance-hardening.md` | ADR-0047: Security & Performance Hardening for v0.2.0 |
| `plans/.archive/0048-wasm-pack-bulk-memory-fix.md` | ADR-0048: Fix wasm-pack Build Bulk Memory Error |
| `plans/.archive/0049-release-checklist.md` | ADR-0049: Release Checklist and Version Sync Protocol |
| `plans/.archive/0050-npm-node24-token-fallback.md` | ADR-0050: npm Publishing - Node.js 24 + Token Fallback |
| `plans/.archive/0051-real-world-readiness.md` | ADR-0051: Real-World Readiness & Quality Hardening |
| `plans/.archive/0053-wave15-api-hardening-and-features.md` | ADR-0053: Wave 15: API Hardening, Missing Features & New Capabilities |
| `plans/.archive/0054-high-impact-new-features.md` | ADR-0054: High-Impact New Features: Text Encoding, Filtered Search, Graph Traversal |
| `plans/.archive/0055-wave16-production-polish.md` | ADR-0055: Wave 16: Production Polish & Correctness |
| `plans/.archive/0056-performance-follow-up-priorities.md` | ADR-0056: Performance Follow-up Priorities After v0.2.0 |
| `plans/.archive/ADR-0030-cli-crate-architecture.md` | ADR-0030: CLI Crate Architecture |
| `plans/.archive/ADR-0052-pre-release-validation-automation.md` | ADR-0052: Pre-Release Validation Automation |

## 2026-08-08 archive (ADR-0097)

| File | What it is |
|------|------------|
| `plans/.archive/2026-08-08-historical/GOAP_STATE_2026_08_08.md` | Verbatim 1,892-line world state before compaction (all per-wave/PR narrative 2026-02 → 2026-08) |
| `plans/.archive/2026-08-08-historical/ACTIONS_2026_08_08.md` | Verbatim 5,106-line action list before compaction (292 complete + 8 active) |

Since 2026-08-08, `GOAP_STATE.md` holds current truth only and `ACTIONS.md`
holds the active queue only; historical keys/actions resolve via these
snapshots or git history.

## Active plan set (do not archive without re-audit)

| Path | Role |
|------|------|
| `plans/README.md` | Index of active planning docs |
| `plans/GOAP_STATE.md` | Canonical world state (YAML) |
| `plans/ACTIONS.md` | Action queue (complete + queued) |
| `plans/GOALS.md` | Primary / engineering goals |
| `plans/GOAP_ORCHESTRATOR.md` | Orchestrator runbook |
| `plans/GOAP_AUDIT_2026_07_14.md` | Wave 32 roadmap (still in progress) |
| `plans/RECOMMENDATIONS_2026_07_20.md` | Current recommendations (this analysis) |
| `plans/ADR_REGISTRY.md` | ADR index |
| `plans/adr/` | Architecture decision records |
| `plans/ARCHIVE_MANIFEST.md` | This file |
| `plans/.archive/` | Superseded ADR copies + dated historical snapshots (2026-07-20, 2026-08-08) |

## Archived in `2026-07-20-historical/completed-goap/` (25 files)

| File | Why archived |
|------|----------------|
| `plans/.archive/2026-07-20-historical/completed-goap/benchmark_optimization_actions.md` | Phase plan complete |
| `plans/.archive/2026-07-20-historical/completed-goap/benchmark_optimization_plan.md` | Phase plan complete |
| `plans/.archive/2026-07-20-historical/completed-goap/DEPENDABOT_ALERTS.md` | Snapshot; live source is GH Dependabot |
| `plans/.archive/2026-07-20-historical/completed-goap/GAP_ANALYSIS_2026_04_30.md` | Superseded by later audits |
| `plans/.archive/2026-07-20-historical/completed-goap/GAP_ANALYSIS_2026_06_26.md` | Wave 30 snapshot; complete |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_ANALYSIS_2026_04_25.md` | Historical analysis |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_BENCHMARK_SUITE.md` | Suite exists; evidence work is in ACTIONS |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_CI_REMEDIATION_MUTATION_PR363.md` | One-shot CI fix complete |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_CI_REMEDIATION_PR356.md` | One-shot CI fix complete |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_CLI_EXAMPLES.md` | Implementation complete |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_CLIPPY_BEST_PRACTICES.md` | Implementation complete |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_COVERAGE_IMPROVEMENT.md` | Coverage gaps closed |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_DUCKDB_COMPANION_CRATE.md` | Companion crate shipped |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_LIFECYCLE_VERIFICATION_FOLLOWUP.md` | Follow-up complete / superseded |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_MAP_PAPER_ANALYSIS.md` | Research note; not active work |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_PRE_EXISTING_ISSUES_PR356.md` | One-shot remediation complete |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_SEMANTIC_BRIDGE.md` | Bridge shipped (ADR-0061) |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_SKILL_MEMORY_HARDENING.md` | Hardening landed |
| `plans/.archive/2026-07-20-historical/completed-goap/GOAP_WORKSPACE_COMPLETION.md` | Workspace extract snapshot |
| `plans/.archive/2026-07-20-historical/completed-goap/swarm_audit_github_2026.md` | Historical swarm audit |
| `plans/.archive/2026-07-20-historical/completed-goap/SWARM_COORDINATION.md` | Historical coordination log |
| `plans/.archive/2026-07-20-historical/completed-goap/UNMAINTAINED_CRATES.md` | Snapshot; use `cargo deny` / deny.toml |
| `plans/.archive/2026-07-20-historical/completed-goap/VERIFICATION_2026_04_29.md` | Dated verification snapshot |
| `plans/.archive/2026-07-20-historical/completed-goap/VERIFICATION_2026_04_30.md` | Dated verification snapshot |
| `plans/.archive/2026-07-20-historical/completed-goap/WAVE_21_P0_COMPLETION.md` | Wave complete |

## Archived in `2026-07-20-historical/handoffs/` (49 files)

Wave W1–W20+ agent handoffs (`analysis_*`, `W*_*.md`, coordination notes),
listed per file because the checker matches per file. Titles are each file's
own H1. Restore path if needed:
`plans/.archive/2026-07-20-historical/handoffs/<name>`.

| File | Title (from its H1) |
|------|---------------------|
| `plans/.archive/2026-07-20-historical/handoffs/analysis_error_handling.md` | Error Handling Analysis Report |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_group_a_testing.md` | Swarm Group A (Testing & Quality) - Analysis Report |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_group_b_performance.md` | Swarm Group B (Performance) - Comprehensive Analysis Report |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_group_c_docs.md` | Swarm Group C (Observability & DX) Analysis Report |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_group_d_features.md` | Swarm Group D Analysis: Advanced Features Assessment |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_logging.md` | Logging & Observability Analysis Report |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_memory_leaks.md` | Memory Leak Prevention Analysis Report |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_performance.md` | Performance Analysis Report - Chaotic Semantic Memory |
| `plans/.archive/2026-07-20-historical/handoffs/analysis_security.md` | Security Analysis Report: chaotic_semantic_memory |
| `plans/.archive/2026-07-20-historical/handoffs/MASTER_ANALYSIS_COORDINATION.md` | Master Analysis Coordination Report |
| `plans/.archive/2026-07-20-historical/handoffs/perf_analysis_batch_similarity_2026_02_20.md` | Swarm Analysis: Performance Benchmark Issues (Groups B, C) |
| `plans/.archive/2026-07-20-historical/handoffs/swarm_analysis_cargo_upgrade_2026.md` | Analysis Swarm: Cargo.toml & Rust Edition 2024 Upgrade |
| `plans/.archive/2026-07-20-historical/handoffs/W10_CI_FIX_FINAL.md` | Handoff: CI Fix and GOAP Sync (Wave 10 Final) |
| `plans/.archive/2026-07-20-historical/handoffs/W11_Final_Analysis_Production_Ready.md` | Comprehensive Codebase Analysis - All Waves Complete |
| `plans/.archive/2026-07-20-historical/handoffs/W11_Release_Engineering_Complete.md` | Wave 11 Handoff: Release Engineering Complete |
| `plans/.archive/2026-07-20-historical/handoffs/W12_CLI_Edge_Case_Examples.md` | Handoff: CLI Edge Case Examples Complete |
| `plans/.archive/2026-07-20-historical/handoffs/W12_CSM_CLI_Dogfooding.md` | Handoff: Skill Memory via CLI (Dogfooding CSM) |
| `plans/.archive/2026-07-20-historical/handoffs/W12_CSM_Skill_Integration_Analysis.md` | Analysis Complete: CSM Integration for opencode CLI Skills |
| `plans/.archive/2026-07-20-historical/handoffs/W12_SKILL_MEMORY_VALIDATION.md` | ✅ Skill Memory System - Validation Complete |
| `plans/.archive/2026-07-20-historical/handoffs/W12b_SKILL_MEMORY_SECURITY_HARDENING.md` | Handoff: Skill-Memory Security Hardening Complete |
| `plans/.archive/2026-07-20-historical/handoffs/W17_Phase48_Performance_Followup.md` | Wave 17 Handoff: Phase 48 Performance Follow-up |
| `plans/.archive/2026-07-20-historical/handoffs/W1_A_to_B_fuzz_findings.md` | W1 A -> B: Fuzz Findings |
| `plans/.archive/2026-07-20-historical/handoffs/W1_B_to_D_perf_and_layout_notes.md` | W1 B -> D: Performance and Layout Notes |
| `plans/.archive/2026-07-20-historical/handoffs/W1_C_to_All_tracing_conventions.md` | W1 C -> All: Tracing Conventions |
| `plans/.archive/2026-07-20-historical/handoffs/W1_D_to_All_schema_constraints.md` | W1 D -> All: Schema Constraints |
| `plans/.archive/2026-07-20-historical/handoffs/W20_A_to_D_impl_contract.md` | Handoff: Group A -> Group D (Wave 20 Implementation) |
| `plans/.archive/2026-07-20-historical/handoffs/W20_B_to_D_fix_contract.md` | Handoff: Group B -> Group D (Wave 20 Fix/Security) |
| `plans/.archive/2026-07-20-historical/handoffs/W20_C_to_D_perf_contract.md` | Handoff: Group C -> Group D (Wave 20 Performance) |
| `plans/.archive/2026-07-20-historical/handoffs/W20_E_to_All_planning_contract.md` | Handoff: Group E -> All (Wave 20 Planning Contract) |
| `plans/.archive/2026-07-20-historical/handoffs/W20_F_to_All_ci_blockers.md` | Handoff: Group F -> All (Wave 20 CI/Release Blockers) |
| `plans/.archive/2026-07-20-historical/handoffs/W5_A_to_B_turso_latency_profile.md` | W5 A -> B Handoff: Turso Latency Profile |
| `plans/.archive/2026-07-20-historical/handoffs/W5_B_to_D_memory_budget_report.md` | W5 B -> D Handoff: Memory Budget Report |
| `plans/.archive/2026-07-20-historical/handoffs/W5_C_to_D_wasm_size_report.md` | W5 C -> D Handoff: WASM Size Report |
| `plans/.archive/2026-07-20-historical/handoffs/W5_D_to_All_performance_gate_decision.md` | W5 D -> All Handoff: Performance Gate Decision |
| `plans/.archive/2026-07-20-historical/handoffs/W6_A_to_All_testing_closure.md` | Wave 6 Handoff: Group A (Testing) → All Groups |
| `plans/.archive/2026-07-20-historical/handoffs/W6_B_to_All_performance_closure.md` | Wave 6 Handoff: Group B (Performance) → All Groups |
| `plans/.archive/2026-07-20-historical/handoffs/W6_C_to_All_observability_closure.md` | Wave 6 Handoff: Group C (Observability) → All Groups |
| `plans/.archive/2026-07-20-historical/handoffs/W6_D_to_All_features_closure.md` | Wave 6 Handoff: Group D (Advanced Features) → All Groups |
| `plans/.archive/2026-07-20-historical/handoffs/W7_A_to_All_testing_pragmatism.md` | W7 Group A -> All: Testing Pragmatism Closure |
| `plans/.archive/2026-07-20-historical/handoffs/W7_B_to_All_documentation_dx.md` | W7 Group B -> All: Documentation & DX Closure |
| `plans/.archive/2026-07-20-historical/handoffs/W7_C_to_All_observability_completion.md` | W7 Group C -> All: Observability Completion |
| `plans/.archive/2026-07-20-historical/handoffs/W7_coordination.md` | Wave 7: Documentation & Parity Improvements |
| `plans/.archive/2026-07-20-historical/handoffs/W7_D_to_All_wasm_parity.md` | W7 Group D -> All: WASM Parity Completion |
| `plans/.archive/2026-07-20-historical/handoffs/W8_A_to_All_batch_tests.md` | Wave 8 Group A: Batch Operations Tests |
| `plans/.archive/2026-07-20-historical/handoffs/W8_B_to_All_crud_tests.md` | Wave 8 Group B: Persistence CRUD Tests |
| `plans/.archive/2026-07-20-historical/handoffs/W8_C_to_All_persistence_benchmarks.md` | Wave 8 Group C: Persistence Benchmarks |
| `plans/.archive/2026-07-20-historical/handoffs/W8_D_to_All_github_standards.md` | Wave 8 Group D: 2026 GitHub Standards |
| `plans/.archive/2026-07-20-historical/handoffs/W9_CLI_design.md` | CLI Module Design Document |
| `plans/.archive/2026-07-20-historical/handoffs/W9_CLI_edge_cases.md` | CLI Edge Cases Documentation |

## Redirect rule for agents

1. Prefer **active** files above for current work.
2. If a skill or doc links an archived path, resolve via this manifest.
3. Do **not** bulk-delete archives; keep git history + this directory.
4. New dated analyses go under `plans/` only while active; archive when complete.
5. When you move a file into `plans/.archive/` — by hand or via
   `scripts/plans-manager.sh archive adr` — add its row here in the same commit.
   `./scripts/validate.sh` fails otherwise, and the checker prints the
   paste-ready row.

## Reference audit notes

Inbound references to archived filenames may still appear in:

- `plans/GOAP_STATE.md` / `plans/ACTIONS.md` (historical notes — intentional)
- Old commit messages / PR bodies
- Skill or progress docs that cite wave handoffs

Those references remain valid via `plans/.archive/2026-07-20-historical/…`.
