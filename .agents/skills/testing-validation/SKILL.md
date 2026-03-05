---
name: testing-validation
description: "Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, and benchmarks. Use when asked to validate, check, or verify the build."
---

# Testing Validation

## Quick Validation
Run `scripts/validate.sh` for the full gate sequence.

## Gate Sequence (manual)
```bash
# Minimal output mode (2026 best practice)
export CARGO_TERM_PROGRESS_WHEN=never

cargo check --message-format=short
cargo test --all-features --quiet  # or: cargo nextest run --all-features
cargo fmt --check
cargo clippy -- -D warnings
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

## Integration Test Files

Run tests by file:
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
Run `scripts/check-docs-links.sh` to validate:
- Internal file links (`@file.md` and `[text](./path.md)` style)
- External URLs (with `--check-urls` flag)
- Code block commands in bash/shell blocks
- Version references consistency across ALL files:
  - Core: Cargo.toml, Cargo.lock, wasm/package.json
  - Docs: README.md, book/src/getting-started.md, CHANGELOG.md, llms.txt
  - Tests: examples/cli/*.sh, tests/*.rs
  - Generated: export.json, csm_test.json

```bash
./scripts/check-docs-links.sh           # Quick check (no URL validation)
./scripts/check-docs-links.sh --fix     # Auto-fix version mismatches
./scripts/check-docs-links.sh --check-urls  # Full check with URL validation
```

## Configurability Check
- Reject hardcoded tunables in new code paths.
- Require named constants and/or env/config-backed settings for thresholds, limits, and sample sizes.

## Known Test Gotchas
- Reservoir tests use `new_seeded(..., 42)` for determinism — don't use `new()` in tests.
- Persistence tests need `tempfile::NamedTempFile` for DB path.
- Criterion closures must not capture mutable state by reference.
