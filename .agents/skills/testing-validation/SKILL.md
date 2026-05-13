---
name: testing-validation
description: "Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, property-based testing, fuzzing, and benchmarks. Use when asked to validate, check, or verify the build."
---

# Testing & Validation

Comprehensive validation including property-based testing, fuzzing, and edge case coverage.

## Quick Validation
Run `scripts/validate.sh` for the full gate sequence.

## Gate Sequence (manual)
```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo check --message-format=short
cargo test --all-features --quiet
cargo fmt --check
cargo clippy -- -D warnings
```

Then check LOC limits with `scripts/loc-check.sh`.

## Property-Based Testing (proptest)

### Key Properties to Test

**HVec10240:**
- `from_bytes(to_bytes(v)) == v` (roundtrip)
- `cosine_similarity(v, v) == 1.0` (self-similarity)
- `cosine_similarity(a, b) == cosine_similarity(b, a)` (symmetry)
- `cosine_similarity(a, b)` in `[-1.0, 1.0]` (bounds)

**Reservoir:**
- `reset()` clears state to zeros
- `step()` with same input produces same output after `reset()`
- `to_hypervector()` fails if `size < 10240`

**Persistence:**
- `save_concept(c); load_concept(c.id) == Some(c)` (roundtrip)
- `delete_concept(id); load_concept(id) == None` (deletion)
- FK constraints reject invalid associations

### Commands
```bash
cargo test --test property_based
cargo fuzz run fuzz_hvec_from_bytes   # requires cargo-fuzz
```

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
| Property | `tests/property_based.rs` | Invariants, roundtrips, bounds |
| Fuzz | `fuzz/` | Adversarial inputs, edge cases |
| Benchmarks | `benches/benchmark.rs` | Performance targets (reservoir_step < 100μs @ 50k) |

## Integration Test Files
```bash
cargo test --test <test_name>
```

Use separate test files in `tests/` for:
- Property-based tests
- Batch operation tests
- Persistence tests
- Framework lifecycle tests
- Edge case coverage

## LOC Enforcement
Every file in `src/*.rs` must be ≤ 500 lines. Run `scripts/loc-check.sh` to verify.

## Documentation Link & Command Validation
```bash
./scripts/check-docs-links.sh           # Quick check (no URL validation)
./scripts/check-docs-links.sh --fix     # Auto-fix version mismatches
./scripts/check-docs-links.sh --check-urls  # Full check with URL validation
```

## Configurability Check
- Reject hardcoded tunables in new code paths.
- Require named constants and/or env/config-backed settings.

## Known Test Gotchas
- Reservoir tests use `new_seeded(..., 42)` for determinism — don't use `new()` in tests.
- Persistence tests need `tempfile::NamedTempFile` for DB path.
- Criterion closures must not capture mutable state by reference.
