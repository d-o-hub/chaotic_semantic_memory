# ADR-0030: Test and Benchmark Gap Remediation

## Status
Proposed

## Context and Problem Statement

A comprehensive codebase analysis on 2026-02-18 revealed significant gaps in test coverage (42 tests, ~60% API coverage) and benchmark coverage (8 benchmarks, 17% coverage). This impacts production confidence and the ability to detect performance regressions.

## Decision Drivers

- Production readiness requires comprehensive test coverage
- Performance goals (turso_roundtrip_under_20ms) lack benchmark verification
- Batch operations have no tests despite being public APIs
- CRUD operations on persistence layer need real database tests
- 2026 GitHub best practices require community health files

## Considered Options

1. **Incremental**: Add tests as bugs are discovered
2. **Comprehensive**: Create missing tests and benchmarks in coordinated swarm wave
3. **Automated**: Use property-based testing exclusively

## Decision Outcome

Chosen option: "Comprehensive", because production systems require proactive test coverage, not reactive fixes.

### Positive Consequences

- Detect regressions before production
- Verify performance targets with benchmarks
- Improve contributor confidence
- Meet 2026 community standards

### Negative Consequences

- Development time investment
- More test code to maintain

## Gaps Identified

### Tests (High Priority)

| Missing Test | API | File:Line |
|-------------|-----|-----------|
| Batch inject | `inject_concepts()` | framework_ops.rs:12 |
| Batch associate | `associate_many()` | framework_ops.rs:38 |
| Batch probe | `probe_batch()` | framework_ops.rs:62 |
| Get concept | `get_concept()` | framework.rs:239 |
| Load merge | `load_merge()` | framework.rs:298 |
| Batch save | `save_concepts()` | persistence.rs:166 |
| Clear all | `clear_all()` | persistence_ops.rs:50 |
| Export JSON | `export_json()` | framework_ops.rs:90 |
| Export binary | `export_binary()` | framework_ops.rs:151 |

### Benchmarks (High Priority)

| Missing Benchmark | Category | Notes |
|-------------------|----------|-------|
| CRUD save | Persistence | `save_concept()` |
| CRUD load | Persistence | `load_concept()` |
| CRUD delete | Persistence | `delete_concept()` |
| hvec_bundle | Hypervector | Critical for concept aggregation |
| Batch operations | Framework | `inject_concepts`, `probe_batch` |

### Missing Implementations

| Gap | File | Priority |
|-----|------|----------|
| `Debug` on Singularity, Reservoir, ChaoticReservoir, Persistence | Multiple | Medium |
| Method docs in framework_ops.rs | framework_ops.rs | Medium |
| singularity.rs LOC overflow (501 lines) | singularity.rs | Low |

### 2026 GitHub Files

| File | Purpose |
|------|---------|
| README.md badges | Discoverability |
| PR template | Quality control |
| llms-full.txt | AI tool integration |
| Issue templates | Community standards |

## Implementation Plan

### Wave 8: Test & Benchmark Remediation

| Group | Focus | Actions |
|-------|-------|---------|
| A | Batch Tests | Add tests for inject_concepts, associate_many, probe_batch |
| B | CRUD Tests | Add persistence CRUD tests with real libsql |
| C | Benchmarks | Add persistence and bundle benchmarks |
| D | GitHub Files | Create badges, templates, llms-full.txt |
| E | Fixes | Debug impls, docs, LOC fix |
