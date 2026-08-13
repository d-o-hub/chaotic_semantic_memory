# ADR Registry

> **Note**: ADRs 0001-0056 were backfilled on 2026-05-01 from ACTIONS.md, git history, and handoff files.
> See ADR-0076 for the backfill process.

## Quick Reference

| ADR | Title | Status | File |
|-----|-------|--------|------|
| 0001 | Use libSQL for Persistence | Accepted | [adr/0001-use-libsql-for-persistence.md](adr/0001-use-libsql-for-persistence.md) |
| 0002 | Hypervector Size (10240 bits) | Accepted | [adr/0002-hypervector-size-10240-bits.md](adr/0002-hypervector-size-10240-bits.md) |
| 0003 | _Superseded by ADR-0008_ | Superseded | N/A |
| 0004 | Sparse Reservoir Matrix | Accepted | [adr/0004-sparse-reservoir-matrix.md](adr/0004-sparse-reservoir-matrix.md) |
| 0005 | Persistence Connection Model | Accepted | [adr/0005-persistence-connection-model.md](adr/0005-persistence-connection-model.md) |
| 0006 | Persistence Batch Operations | Accepted | [adr/0006-persistence-batch-operations.md](adr/0006-persistence-batch-operations.md) |
| 0007 | Similarity Search Optimization | Accepted | [adr/0007-similarity-search-optimization.md](adr/0007-similarity-search-optimization.md) |
| 0008 | WASM Rayon Gating | Accepted | [adr/0008-wasm-rayon-gating.md](adr/0008-wasm-rayon-gating.md) |
| 0009 | Partial Reservoir Updates | Accepted | [adr/0009-partial-reservoir-updates.md](adr/0009-partial-reservoir-updates.md) |
| 0010 | Public API Result Contract | Accepted | [adr/0010-public-api-result-contract.md](adr/0010-public-api-result-contract.md) |
| 0011 | SQLite Foreign Keys & Builder Migration | Accepted | [adr/0011-sqlite-foreign-keys-and-builder-migration.md](adr/0011-sqlite-foreign-keys-and-builder-migration.md) |
| 0012 | ConceptBuilder Metadata Error Propagation | Accepted | [adr/0012-conceptbuilder-metadata-error-propagation.md](adr/0012-conceptbuilder-metadata-error-propagation.md) |
| 0013 | SIMD Hypervector Operations | Accepted | [adr/0013-simd-hypervector-operations.md](adr/0013-simd-hypervector-operations.md) |
| 0014 | Connection Pooling for Turso | Accepted | [adr/0014-connection-pooling-for-turso.md](adr/0014-connection-pooling-for-turso.md) |
| 0015 | Structured Logging | Accepted | [adr/0015-structured-logging.md](adr/0015-structured-logging.md) |
| 0016 | Export/Import Migration | Accepted | [adr/0016-export-import-migration.md](adr/0016-export-import-migration.md) |
| 0017 | Concept Versioning | Accepted | [adr/0017-concept-versioning.md](adr/0017-concept-versioning.md) |
| 0018 | Input Validation Policy | Accepted | [adr/0018-input-validation-policy.md](adr/0018-input-validation-policy.md) |
| 0019 | Backup/Restore Safety | Accepted | [adr/0019-backup-restore-safety.md](adr/0019-backup-restore-safety.md) |
| 0020 | Silent Data Loss on Load | Accepted | [adr/0020-silent-data-loss-on-load.md](adr/0020-silent-data-loss-on-load.md) |
| 0021 | Auto Schema Migration | Accepted | [adr/0021-auto-schema-migration.md](adr/0021-auto-schema-migration.md) |
| 0022 | WASM API Parity (Original) | Accepted | [adr/0022-wasm-api-parity-original.md](adr/0022-wasm-api-parity-original.md) |
| 0023 | Zero-Alloc Query Cache | Accepted | [adr/0023-zero-alloc-query-cache.md](adr/0023-zero-alloc-query-cache.md) |
| 0024 | Concept Expiration (TTL) | Implemented (lifecycle follow-up queued) | [adr/0024-concept-expiration-ttl.md](adr/0024-concept-expiration-ttl.md) |
| 0025 | Weighted Forgetting (Decay) | Implemented | [adr/0025-weighted-forgetting-decay.md](adr/0025-weighted-forgetting-decay.md) |
| 0026 | Namespace Isolation | Implemented | [adr/0026-namespace-isolation.md](adr/0026-namespace-isolation.md) |
| 0027 | Documentation Standards | Implemented | [adr/0027-documentation-standards.md](adr/0027-documentation-standards.md) |
| 0028 | Observability Completion | Implemented | [adr/0028-observability-completion.md](adr/0028-observability-completion.md) |
| 0029 | WASM API Parity (Phase 2) | Implemented | [adr/0029-wasm-api-parity-phase-2.md](adr/0029-wasm-api-parity-phase-2.md) |
| 0030 | Test & Benchmark Gap Remediation | Implemented | [adr/0030-test-and-benchmark-gap-remediation.md](adr/0030-test-and-benchmark-gap-remediation.md) |
| 0031 | Two-Tier Architecture Documentation | Accepted | [adr/0031-two-tier-architecture-documentation.md](adr/0031-two-tier-architecture-documentation.md) |
| 0032 | CLI Robustness | Implemented | [adr/0032-cli-robustness.md](adr/0032-cli-robustness.md) |
| 0033 | WASM Panic Safety | Implemented | [adr/0033-wasm-panic-safety.md](adr/0033-wasm-panic-safety.md) |
| 0034 | Framework Metadata Injection | Implemented | [adr/0034-framework-metadata-injection.md](adr/0034-framework-metadata-injection.md) |
| 0035 | Cache Memory Guardrails | Implemented | [adr/0035-cache-memory-guardrails.md](adr/0035-cache-memory-guardrails.md) |
| 0036 | CI/DX Hardening | Implemented | [adr/0036-ci-dx-hardening.md](adr/0036-ci-dx-hardening.md) |
| 0037 | Rust Best Practices | Implemented | [adr/0037-rust-best-practices.md](adr/0037-rust-best-practices.md) |
| 0038 | Cargo.toml Modernization | Implemented | [adr/0038-cargo-toml-modernization.md](adr/0038-cargo-toml-modernization.md) |
| 0039 | Release Engineering | Implemented | [adr/0039-release-engineering.md](adr/0039-release-engineering.md) |
| 0040 | Async Lock Safety | Implemented | [adr/0040-async-lock-safety.md](adr/0040-async-lock-safety.md) |
| 0041 | Batch Similarity Optimization | Implemented | [adr/0041-batch-similarity-optimization.md](adr/0041-batch-similarity-optimization.md) |
| 0042 | Release Automation v0.1.0 | Accepted | [adr/0042-release-automation-v010.md](adr/0042-release-automation-v010.md) |
| 0043 | Skill-Memory Security Hardening | Accepted | [adr/0043-skill-memory-security-hardening.md](adr/0043-skill-memory-security-hardening.md) |
| 0044 | Memory Limits and Resource Governance | Implemented | [adr/0044-memory-limits-and-resource-governance.md](adr/0044-memory-limits-and-resource-governance.md) |
| 0045 | Security Policy for Input Validation | Implemented | [adr/0045-security-policy-for-input-validation.md](adr/0045-security-policy-for-input-validation.md) |
| 0046 | npm OIDC Trusted Publishing | Implemented | [adr/0046-npm-oidc-trusted-publishing.md](adr/0046-npm-oidc-trusted-publishing.md) |
| 0047 | Security & Performance Hardening | Implemented | [adr/0047-security-and-performance-hardening.md](adr/0047-security-and-performance-hardening.md) |
| 0048 | WASM-pack Bulk Memory Fix | Implemented | [adr/0048-wasm-pack-bulk-memory-fix.md](adr/0048-wasm-pack-bulk-memory-fix.md) |
| 0050 | npm Node.js 24 + Token Fallback | Implemented | [adr/0050-npm-nodejs-24-and-token-fallback.md](adr/0050-npm-nodejs-24-and-token-fallback.md) |
| 0051 | Real-World Readiness & Quality Hardening | Implemented | [adr/0051-real-world-readiness-and-quality-hardening.md](adr/0051-real-world-readiness-and-quality-hardening.md) |
| 0053 | API Hardening & New Features | Implemented | [adr/0053-api-hardening-and-new-features.md](adr/0053-api-hardening-and-new-features.md) |
| 0054 | High-Impact New Features | Implemented | [adr/0054-high-impact-new-features.md](adr/0054-high-impact-new-features.md) |
| 0055 | Production Polish & Correctness | Implemented | [adr/0055-production-polish-and-correctness.md](adr/0055-production-polish-and-correctness.md) |
| 0056 | Performance Follow-up Priorities | Implemented | [adr/0056-performance-follow-up-priorities.md](adr/0056-performance-follow-up-priorities.md) |
| 0057 | Phase 41 API Completion and Memory Events | Implemented | [adr/0057-phase41-api-completion-and-events.md](adr/0057-phase41-api-completion-and-events.md) |
| 0058 | Fix Import/Export Serialization | Implemented | [adr/0058-fix-import-export-serialization.md](adr/0058-fix-import-export-serialization.md) |
| 0059 | Retrieval Optimization and Benchmark Hygiene | Implemented | [adr/0059-retrieval-optimization.md](adr/0059-retrieval-optimization.md) |
| 0060 | Configurable Hypervector Dimensions | Deferred | [adr/0060-configurable-dimensions.md](adr/0060-configurable-dimensions.md) |
| 0061 | Semantic Bridge Layer | Implemented | [adr/0061-semantic-bridge-layer.md](adr/0061-semantic-bridge-layer.md) |
| 0062 | Hybrid BM25-HDC Retrieval | Implemented | [adr/0062-hybrid-bm25-hdc-retrieval.md](adr/0062-hybrid-bm25-hdc-retrieval.md) |
| 0063 | Database Table Prefix | Implemented | [adr/0063-database-table-prefix.md](adr/0063-database-table-prefix.md) |
| 0066 | CLI ↔ Framework API Parity | Implemented | [adr/0066-cli-framework-api-parity.md](adr/0066-cli-framework-api-parity.md) |
| 0067 | MCP Server | Implemented | [adr/0067-mcp-server.md](adr/0067-mcp-server.md) |
| 0068 | HNSW ANN Index | Implemented | [adr/0068-hnsw-ann-index.md](adr/0068-hnsw-ann-index.md) |
| 0069 | Embedding Model Bridge | Implemented | [adr/0069-embedding-model-bridge.md](adr/0069-embedding-model-bridge.md) |
| 0070 | GraphRAG Hybrid Retrieval | Implemented | [adr/0070-graphrag-hybrid-retrieval.md](adr/0070-graphrag-hybrid-retrieval.md) |
| 0071 | Reranking MMR Pipeline | Implemented | [adr/0071-reranking-mmr-pipeline.md](adr/0071-reranking-mmr-pipeline.md) |
| 0072 | OTLP Exporter | Implemented | [adr/0072-otlp-exporter.md](adr/0072-otlp-exporter.md) |
| 0073 | Namespace Isolation | Implemented | [adr/0073-namespace-isolation.md](adr/0073-namespace-isolation.md) |
| 0074 | Version History Surface | Implemented | [adr/0074-version-history-surface.md](adr/0074-version-history-surface.md) |
| 0075 | Quantized Binary Hypervectors | Implemented | [adr/0075-quantized-binary-hypervectors.md](adr/0075-quantized-binary-hypervectors.md) |
| 0076 | ADR Backfill | Implemented | [adr/0076-adr-backfill.md](adr/0076-adr-backfill.md) |
| 0077 | Clippy Pedantic Selective Promotion | Phase A+B Implemented | [adr/0077-clippy-pedantic-selective-promotion.md](adr/0077-clippy-pedantic-selective-promotion.md) |
| 0078 | CloudEvents Event Emitter | Implemented | [adr/0078-cloudevents-event-emitter.md](adr/0078-cloudevents-event-emitter.md) |
| 0079 | DuckDB Companion Crate — Workspace Restructure | Implemented | [adr/0079-duckdb-companion-crate-workspace.md](adr/0079-duckdb-companion-crate-workspace.md) |
| 0080 | DuckDB Companion — Phase 1: Read-Only Analytics | Implemented | [adr/0080-duckdb-phase1-readonly-analytics.md](adr/0080-duckdb-phase1-readonly-analytics.md) |
| 0081 | DuckDB Companion — Phase 2: Parquet Export | Implemented | [adr/0081-duckdb-phase2-parquet-export.md](adr/0081-duckdb-phase2-parquet-export.md) |
| 0082 | DuckDB Companion — Phase 3: Optional CLI Integration | Implemented | [adr/0082-duckdb-phase3-cli-integration.md](adr/0082-duckdb-phase3-cli-integration.md) |
| 0083 | Memory Lifecycle Verification & Export Format | Accepted | [adr/0083-memory-lifecycle-verification-and-export-format.md](adr/0083-memory-lifecycle-verification-and-export-format.md) |
| 0084 | GOAP Reconciliation and Codebase Alignment | Accepted | [adr/0084-goap-reconciliation.md](adr/0084-goap-reconciliation.md) |
| 0085 | GOAP Reconciliation 2026-06 | Accepted | [adr/0085-goap-reconciliation-2026-06.md](adr/0085-goap-reconciliation-2026-06.md) |
| 0086 | OTLP / Prometheus Implementation Notes | Implemented | [adr/0086-otlp-prom-implementation.md](adr/0086-otlp-prom-implementation.md) |
| 0087 | CI Failure Remediation for PR #356 (Workspace Split) | Implemented | [adr/0087-ci-failure-remediation-pr356.md](adr/0087-ci-failure-remediation-pr356.md) |
| 0088 | Pre-existing Issues Documented During PR #356 Codacy Remediation | Accepted | [adr/0088-pre-existing-issues-pr356-codacy-remediation.md](adr/0088-pre-existing-issues-pr356-codacy-remediation.md) |
| 0089 | GOAP Reconciliation 2026-06-16 | Accepted | [adr/0089-goap-reconciliation-2026-06-16.md](adr/0089-goap-reconciliation-2026-06-16.md) |
| 0090 | Harness Engineering & rust-2026-template Alignment | Accepted | [adr/0090-harness-engineering-template-alignment.md](adr/0090-harness-engineering-template-alignment.md) |
| 0091 | Hyperchaotic Bit-Slicing for Binary Semantic Hashing | Implemented | [adr/0091-hyperchaotic-bit-slicing.md](adr/0091-hyperchaotic-bit-slicing.md) |
| 0092 | GOAP Reconciliation 2026-07-11 | Accepted | [adr/0092-goap-reconciliation-2026-07-11.md](adr/0092-goap-reconciliation-2026-07-11.md) |
| 0093 | Authoritative Persistence and Derived ANN Index Consistency | Accepted | [adr/0093-authoritative-persistence-and-derived-index-consistency.md](adr/0093-authoritative-persistence-and-derived-index-consistency.md) |
| 0094 | Workspace Ownership and Feature Contracts | Accepted | [adr/0094-workspace-ownership-and-feature-contracts.md](adr/0094-workspace-ownership-and-feature-contracts.md) |
| 0095 | Evidence-Driven Quality Gates | Accepted | [adr/0095-evidence-driven-quality-gates.md](adr/0095-evidence-driven-quality-gates.md) |
| 0096 | Agent Skill and Workflow Validation | Accepted | [adr/0096-agent-skill-and-workflow-validation.md](adr/0096-agent-skill-and-workflow-validation.md) |
| 0097 | GOAP Reconciliation and Plans Compaction 2026-08-08 | Accepted | [adr/0097-goap-reconciliation-plans-compaction-2026-08-08.md](adr/0097-goap-reconciliation-plans-compaction-2026-08-08.md) |
| 0098 | GOAP Reconciliation 2026-08-12 | Accepted | [adr/0098-goap-reconciliation-2026-08-12.md](adr/0098-goap-reconciliation-2026-08-12.md) |

## Status Definitions

- **Proposed**: Decision drafted for maintainer approval; dependent implementation remains queued
- **Accepted**: Decision approved, may be implemented or in progress
- **Implemented**: Code complete and merged
- **Deferred**: Postponed to future release, see ADR for trigger conditions
- **Superseded**: Replaced by newer ADR (noted in header)

## Wave 11 Active ADRs

Per Swarm Consensus 2026-02-19, these ADRs were implemented for release engineering:

1. **ADR-0039**: Release Engineering
   - semantic-release for automated versioning
   - Trusted Publishing for crates.io (OIDC-based)
   - npm provenance for WASM bindings
   - mdBook for GitHub Pages documentation
   - CLI usage examples and documentation

## Wave 10 Active ADRs

Per Swarm Consensus 2026-02-19, these ADRs were implemented for 1.0 release (Phases 21-24):

1. **ADR-0032**: CLI Robustness
   - JSON escaping with serde_json (replaces format!)
   - Proper exit code mapping (1-7 instead of 255)
   - Error output respects --output-format flag
   - Remove unused --config flag

2. **ADR-0033**: WASM Panic Safety
   - Replace Reflect::set().unwrap() with error propagation
   - Convert metrics_snapshot() to return Result
   - Eliminate unrecoverable panics across WASM boundary

3. **ADR-0034**: Framework Metadata Injection
   - Add inject_concept_with_metadata() API
   - Add with_reservoir_input_size() to FrameworkBuilder
   - Add WASM batch API parity (get_concept, inject_concepts, etc.)

4. **ADR-0035**: Cache Memory Guardrails
   - Reduce DEFAULT_CONCEPT_CACHE_SIZE from 1000 to 128
   - Add max_cached_top_k limit (default: 100)
   - Bypass cache when top_k exceeds limit

5. **ADR-0036**: CI/DX Hardening
   - LOC gate recursive fix (find src -name '*.rs')
   - Pre-commit hooks (format + LOC check)
   - Clippy flags alignment between CI and local
   - Post-commit hook fixes (no test + amend)

6. **ADR-0037**: Rust Best Practices
   - #[must_use] annotations on constructors
   - Improved unsafe documentation
   - Targeted clippy suppressions (per-loop, not file-wide)
   - CLI JSON serde usage

7. **ADR-0038**: Cargo.toml Modernization
   - Edition 2024 upgrade with MSRV 1.85
   - crates.io metadata (description, license, repository, keywords, categories)
   - Dependency version pinning for reproducibility
   - CLI deps gating with target-specific dependencies
   - Remove unused exitcode crate

8. **ADR-0040**: Async Lock Safety
   - Restructure lock scopes to avoid holding RwLock across await
   - Fix load_replace, load_merge, import_json, import_binary
   - Eliminate starvation risk during concurrent operations

 ## Wave 18 Active ADRs (Serialization Fixes)

Critical fixes for import/export functionality:

1. **ADR-0058**: Fix Import/Export Serialization
   - Base64 encoding for HVec10240 in JSON format
   - BinaryMetadataValue for bincode-compatible binary export
   - Separate BinaryExportPayload struct for binary format
   - Consistent bincode options for export/import
   - Added turso-memory-verification skill

## Wave 13 Active ADRs (Post-Release Security & Hardening)

 Per specialist analysis swarm findings (2026-02-20), these ADRs address critical security and resource issues:

 1. **ADR-0043**: Skill-Memory Security Hardening
    - Input validation for skill names and concept IDs
    - Path traversal protection in skill-memory.sh
    - Error handling improvements with exit code differentiation

 2. **ADR-0044**: Memory Limits and Resource Governance
    - Configurable max_concepts with safe default (100K)
    - Configurable max_associations per concept (1K default)
    - Version retention limits with hard ceiling (1000 max)
    - Metadata size validation (64KB limit)
    - Query cache size limits

 3. **ADR-0045**: Security Policy for Input Validation
    - Bincode deserialization size limits (100MB max)
    - Path traversal protection for file operations
    - Metadata size validation during concept building
    - Error message sanitization to prevent token leakage
    - Association strength bounds checking

 ## Wave 7 Active ADRs

Per Swarm Consensus 2026-02-17, these ADRs were implemented for 1.0 release:

1. **ADR-0027**: Documentation Standards
   - Document FrameworkConfig, SingularityConfig
   - Expand README with Installation/Configuration
   - Add basic_in_memory example

2. **ADR-0028**: Observability Completion
   - Add tracing to singularity.rs
   - Add cache hit/miss metrics
   - Add reservoir operation metrics

3. **ADR-0029**: WASM API Parity
   - Expose process_sequence() to WASM
   - Add memory-based export/import (Uint8Array)

## Deferred ADRs (Post-1.0)

These ADRs are valid but deferred based on Swarm Consensus:

- **ADR-0060 (Dimensions)**: Configurable hypervector dimensions - defer until user demand

TTL advanced policies and association decay were implemented on 2026-06-23; see ADR-0024 and ADR-0025 resolution notes.

## PR #356 Followup ADRs

- **ADR-0087**: CI Failure Remediation for PR #356 (Workspace Split)
- **ADR-0088**: Pre-existing Issues Documented During PR #356 Codacy Remediation

## GOAP Reconciliation ADRs

- **ADR-0084**: GOAP Reconciliation and Codebase Alignment (2026-05-20)
- **ADR-0085**: GOAP Reconciliation 2026-06 (2026-06-06)
- **ADR-0089**: GOAP Reconciliation 2026-06-16 (post-PR #396 / #389 audit; removed duplicate `action_last_completed`, marked 3 stale "deferred" actions as complete)
- **ADR-0097**: GOAP Reconciliation and Plans Compaction 2026-08-08 (state files split into current-truth vs dated archive snapshots; `ACTIONS.md` is active-queue-only)
- **ADR-0098**: GOAP Reconciliation 2026-08-12 (PR roast wave: #620/#621/#622 landed; BM25 absence wired; wave-33 flags trued; csm-cli/csm-wasm dead dupes deleted; bench harnesses added)

## Links

- Full ADR directory: `plans/adr/`
- GOAP planning: `plans/ACTIONS.md`, `plans/GOAP_STATE.md`
- Swarm Consensus: Analysis artifacts in `plans/handoffs/`
