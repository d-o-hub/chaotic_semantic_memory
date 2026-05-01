# ADR-0030: Test & Benchmark Gap Remediation

## Status

Accepted (backfilled 2026-05-01) - Wave 8 Complete

## Context

Test and benchmark gaps identified:
- Missing edge case coverage
- No critical error path tests
- Benchmark methodology issues
- CI validation gaps

## Decision

Remediate **test and benchmark gaps**.

**Deliverables:**
- Edge case tests (empty sequences, zero-length inputs, max limits)
- Critical error path tests (concept ID boundary, association validation)
- Benchmark methodology (truthful storage metric, report contract)
- CI validation (benchmark workspace tests, WASM smoke tests)

## Consequences

### Positive
- Comprehensive test coverage
- Error paths validated
- Benchmarks reflect actual system
- CI catches regressions

### Negative
- Test maintenance overhead
- Benchmark CI runtime
- Edge case complexity

## Implementation

- Files: tests/edge_cases.rs, tests/critical_error_paths.rs
- Benchmarks: benches/src/runner.rs metrics
- CI: .github/workflows/ci.yml, benchmark-ci.yml

## Sources

- ACTIONS.md lines 489-545 (Phase 5-6 actions)
- W8 handoffs: batch_tests, crud_tests, persistence_benchmarks
- Git: test coverage improvements