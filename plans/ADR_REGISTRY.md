# ADR Registry

## Quick Reference

| ADR | Title | Status | Priority |
|-----|-------|--------|----------|
| 0001 | Use libSQL for Persistence | Accepted | Implemented |
| 0002 | Hypervector Size (10240 bits) | Accepted | Implemented |
| 0004 | Sparse Reservoir Matrix | Accepted | Implemented |
| 0005 | Persistence Connection Model | Accepted | Implemented |
| 0006 | Persistence Batch Operations | Accepted | Implemented |
| 0007 | Similarity Search Optimization | Accepted | Implemented |
| 0008 | WASM Rayon Gating | Accepted | Implemented |
| 0009 | Partial Reservoir Updates | Accepted | Implemented |
| 0010 | Public API Result Contract | Accepted | Implemented |
| 0011 | SQLite Foreign Keys & Builder Migration | Accepted | Implemented |
| 0012 | ConceptBuilder Metadata Error Propagation | Accepted | Implemented |
| 0013 | SIMD Hypervector Operations | Accepted | Implemented |
| 0014 | Connection Pooling for Turso | Accepted | Implemented |
| 0015 | Structured Logging | Accepted | Implemented |
| 0016 | Export/Import Migration | Accepted | Implemented |
| 0017 | Concept Versioning | Accepted | Implemented |
| 0018 | Input Validation Policy | Accepted | Implemented |
| 0019 | Backup/Restore Safety | Accepted | Implemented |
| 0020 | Silent Data Loss on Load | Accepted | Implemented |
| 0021 | Auto Schema Migration | Accepted | Implemented |
| 0022 | WASM API Parity (Original) | Accepted | Implemented |
| 0023 | Zero-Alloc Query Cache | Accepted | Implemented |
| **0027** | **Documentation Standards** | **Implemented** | **Wave 7 - Complete** |
| **0028** | **Observability Completion** | **Implemented** | **Wave 7 - Complete** |
| **0029** | **WASM API Parity (Phase 2)** | **Implemented** | **Wave 7 - Complete** |
| **0030** | **Test & Benchmark Gap Remediation** | **Implemented** | **Wave 8 - Complete** |
| **0038** | **Cargo.toml Modernization** | **Implemented** | **Wave 10 - Phase 24** |
| **0039** | **Release Engineering** | **Implemented** | **Wave 11 - Phase 25** |
| **0031** | **Two-Tier Architecture Documentation** | **Accepted** | **Documentation** |
| **0032** | **CLI Robustness** | **Implemented** | **Wave 10** |
| **0033** | **WASM Panic Safety** | **Implemented** | **Wave 10** |
| **0034** | **Framework Metadata Injection** | **Implemented** | **Wave 10** |
| **0035** | **Cache Memory Guardrails** | **Implemented** | **Wave 10** |
| **0036** | **CI/DX Hardening** | **Implemented** | **Wave 10 - Phase 22** |
| **0037** | **Rust Best Practices** | **Implemented** | **Wave 10 - Phase 23** |
| **0040** | **Async Lock Safety** | **Implemented** | **Wave 10** |
| **0041** | **Batch Similarity Optimization** | **Implemented** | **Wave 12 - Phase 6B** |
| **0043** | **Skill-Memory Security Hardening** | **Accepted** | **Skill System** |
| **0044** | **Memory Limits and Resource Governance** | **Proposed** | **Wave 13 - Phase 27** |
| **0045** | **Security Policy for Input Validation** | **Proposed** | **Wave 13 - Phase 27** |
| **0046** | **npm OIDC Trusted Publishing** | **Proposed** | **Wave 12B - Phase 26B** |
| **0047** | **Security & Performance Hardening** | **Implemented** | **Wave 13 - Phase 27** |
| **0048** | **WASM-pack Bulk Memory Fix** | **Implemented** | **2026-02-27** |
| **0050** | **npm Node.js 24 + Token Fallback** | **Implemented** | **2026-02-28** |
| **0051** | **Real-World Readiness & Quality Hardening** | **Implemented** | **Wave 14 - Phase 29-31** |
| **0053** | **API Hardening & New Features** | **Proposed** | **Wave 15 - Phase 32-36** |
| **0054** | **High-Impact New Features** | **Proposed** | **Wave 15 - Phase 37-41** |
| **0055** | **Production Polish & Correctness** | **Implemented** | **Wave 16 - Phase 42-47** |
| **0056** | **Performance Follow-up Priorities** | **Proposed** | **2026-03-09 Analysis** |
| 0024 | Concept Expiration (TTL) | Deferred | Post-1.0 |
| 0024 | Performance Optimizations Phase 2 | Deferred | Post-1.0 |
| 0025 | Weighted Forgetting (Decay) | Deferred | Post-1.0 |
| 0026 | Namespace Isolation | Deferred | Post-1.0 |

## Status Definitions

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

- **ADR-0024 (TTL)**: Concept expiration - defer until session management use case
- **ADR-0024 (Phase 2)**: SIMD completion, Product Quantization, LSH - defer until >200k concepts
- **ADR-0025 (Decay)**: Association decay - defer until biological modeling requested
- **ADR-0026 (Namespaces)**: Multi-tenancy - defer until SaaS deployment need

## Links

- Full ADR directory: `plans/adr/`
- GOAP planning: `plans/ACTIONS.md`, `plans/GOAP_STATE.md`
- Swarm Consensus: Analysis artifacts in `plans/handoffs/`
