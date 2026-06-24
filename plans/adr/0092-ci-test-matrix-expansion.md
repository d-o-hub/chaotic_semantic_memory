# ADR-0092: CI Test Matrix Expansion

## Status

Proposed

## Context and Problem Statement

The CI `test-workspace-crates` job matrix covers: csm-core, csm-embedding,
csm-memory, csm-retrieval, csm-persistence, csm-traits. Two workspace members
are excluded:

1. **csm-wasm** — tested only via the `wasm` CI job (compile check), not
   unit-tested independently with `cargo test`.
2. **csm-cli** — tested implicitly via the root crate's `--all-features` run
   but has no dedicated job. Its `src/main.rs` and command modules could have
   untested code paths not exercised by integration tests.

## Decision

Expand `.github/workflows/ci.yml` test matrix:

1. Add `csm-cli` to the `test-workspace-crates` matrix with appropriate
   feature flags (`--features cli`).
2. Add a `test-csm-cli-unit` step that runs `cargo test -p csm-cli` to catch
   CLI-specific unit test regressions independently.
3. For `csm-wasm`: add `cargo test -p csm-wasm --target wasm32-unknown-unknown`
   or document why WASM unit tests cannot run in CI (no wasm-bindgen-test
   runner configured). If infeasible, add a comment to the CI file explaining
   the coverage gap.

Estimated cost: 2

## Consequences

- CLI regressions caught earlier with clearer failure attribution.
- Explicit documentation of WASM test limitations.
- Slight CI time increase (~30s per additional matrix entry).

## References

- `.github/workflows/ci.yml` — CI workflow
- `crates/csm-cli/` — CLI workspace member
- `crates/csm-wasm/` — WASM workspace member
