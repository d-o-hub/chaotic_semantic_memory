---
name: testing-validation
description: "Validate the chaotic_semantic_memory crate: compile, test, lint, LOC caps, and benchmarks. Use when asked to validate, check, or verify the build."
---

# Testing Validation

## Required Validation

Run both repository gates after implementation:

```bash
scripts/validate.sh
scripts/harness-check.sh all
```

On a fresh checkout, run `scripts/install-hooks.sh` first. Strict validation
must not silently install hooks.

If the full Cargo test/link step exceeds local memory, use
`scripts/validate-low-memory.sh` for the bounded-memory Rust subset. It does not
replace either required gate or their non-Cargo checks; a failed compiler or
linker remains a failure.

For a nontrivial feature or bug fix, exercise the changed path with a focused
smoke scenario in addition to tests. A passing compile or test suite alone does
not prove the user-visible behavior; report only observed results.

## Proving a merged tree

`scripts/validate.sh` runs `cargo test --all-targets --all-features` **without**
`--no-fail-fast`: the first failing target suppresses every target after it *and*
every text gate that follows the suite. A `--lib` probe is blind the other way —
it never compiles `tests/**`, and the mutation gate scores only the lib target.
So neither "validate.sh is green" nor "--lib is green" certifies a
merge. Measure the whole tree explicitly:

```bash
cargo test --all-targets --all-features --no-fail-fast
grep '^test result:' <the log> | sort | uniq -c
```

When a change replaces a public contract, the lib assertions moving is only half
the job: grep `tests/**` for the old behaviour's *name shape* (`*_skips_*`,
`*_is_ok`, `*_without_*_returns_empty`) and rewrite those in the same commit,
asserting the reported entity/field/value so a later relaxation fails on the
reason rather than on a bare error match.

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
| Unit | `src/**/*.rs`, `crates/*/src/**/*.rs` | Mutation-visible logic, boundaries, errors, and state transitions |
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

Every Rust file under `src/` and `crates/` must be ≤ 500 lines. Run `scripts/loc-check.sh` to verify.

`loc-check.sh` judges each file and knows nothing about the aggregate.
`docs/architecture/context.yaml` → `current_status.modules_under_loc_limit` is compared against a live
`src/**/*.rs` file count by `tests/validate_llm_context.py`, so **adding or removing any `src/*.rs` file
means bumping that counter in the same commit** (measured 2026-10-29: a new `src/framework_decay_guard_
tests.rs` module left the declaration at 114/114 while the tree held 115 files). The pre-commit hook does
not run this gate and `scripts/validate.sh` runs it late and fail-fast — so every stage after it is
unverified too, and the drift costs a whole CI cycle rather than a warning. After adding a file, the
cheap check is `python3 tests/validate_llm_context.py`.

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
