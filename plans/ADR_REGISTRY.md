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
| **0031** | **Two-Tier Architecture Documentation** | **Accepted** | **Documentation** |
| 0024 | Concept Expiration (TTL) | Deferred | Post-1.0 |
| 0024 | Performance Optimizations Phase 2 | Deferred | Post-1.0 |
| 0025 | Weighted Forgetting (Decay) | Deferred | Post-1.0 |
| 0026 | Namespace Isolation | Deferred | Post-1.0 |

## Status Definitions

- **Accepted**: Decision approved, may be implemented or in progress
- **Implemented**: Code complete and merged
- **Deferred**: Postponed to future release, see ADR for trigger conditions
- **Superseded**: Replaced by newer ADR (noted in header)

## Wave 10 Active ADRs

Per Swarm Consensus 2026-02-19, these ADRs are being implemented for 1.0 release:

1. **ADR-0038**: Cargo.toml Modernization
   - Edition 2024 upgrade with MSRV 1.85
   - crates.io metadata (description, license, repository, keywords, categories)
   - Dependency version pinning for reproducibility
   - CLI deps gating with `cli` feature
   - Remove unused exitcode crate

2. **ADR-0036**: CI/DX Hardening
   - LOC gate recursive fix
   - Pre-commit hooks
   - Clippy flags alignment
   - Post-commit hook fixes

3. **ADR-0037**: Rust Best Practices
   - #[must_use] annotations
   - Unsafe docs improvement
   - Clippy suppressions targeting
   - CLI JSON serde usage

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
