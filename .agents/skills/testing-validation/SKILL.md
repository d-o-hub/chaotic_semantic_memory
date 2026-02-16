---
name: testing-validation
description: "Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, and benchmarks. Use when asked to validate, check, or verify the build."
---

# Testing Validation

## Quick Validation
Run `scripts/validate.sh` for the full gate sequence.

## Gate Sequence (manual)
```bash
# Quiet mode - reduced output
cargo check --quiet
cargo test --all-features --quiet
cargo fmt --check --quiet
cargo clippy --quiet -- -D warnings
```

Then check LOC limits with `scripts/loc-check.sh`.

## Benchmark Validation
```bash
# First run: save a baseline
cargo bench --bench benchmark -- --save-baseline main

# Subsequent runs: compare against baseline
cargo bench --bench benchmark -- --baseline main
```

**Do NOT use** `cargo bench -- --baseline` without a name — it fails silently.

## Test Coverage Goals

| Layer | Location | What to test |
|---|---|---|
| Unit | `src/*.rs` `#[cfg(test)]` | Core logic, edge cases, error paths |
| Integration | `tests/*.rs` | Public API behavior, persistence roundtrips |
| Benchmarks | `benches/benchmark.rs` | Performance targets (reservoir_step < 100μs @ 50k) |

## LOC Enforcement
Every file in `src/*.rs` must be ≤ 500 lines. Run `scripts/loc-check.sh` to verify.

## Known Test Gotchas
- Reservoir tests use `new_seeded(..., 42)` for determinism — don't use `new()` in tests.
- Persistence tests need `tempfile::NamedTempFile` for DB path.
- Criterion closures must not capture mutable state by reference.
