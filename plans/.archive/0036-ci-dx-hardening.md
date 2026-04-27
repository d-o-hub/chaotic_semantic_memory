# [ADR-0036] CI/DX Hardening: LOC Gate, Pre-Commit Hook, Clippy Parity

## Status
Proposed

## Context and Problem Statement
Six CI/DX gaps were identified in analysis:

1. **LOC gate only checks `src/*.rs`**: Both `scripts/validate.sh` (line 24) and `.github/workflows/ci.yml` (line 65) use `for file in src/*.rs` which skips 11 files in `src/cli/`, `src/cli/commands/`, and `src/bin/`. Any of those files could exceed 500 LOC without detection.

2. **No pre-commit hook**: Only a `post-commit` hook exists (auto-updates drawio diagram). No `pre-commit` hook runs `cargo fmt --check` or basic validation before commits land locally. The `scripts/validate.sh` exists but must be run manually.

3. **Clippy flag inconsistency**: CI runs `cargo clippy -- -D warnings` (line 48) but local validate.sh runs `cargo clippy --all-targets -- -D warnings` (line 11). CI misses test/bench target lints; local misses `--all-features` lints.

4. **Post-commit hook runs tests and amends commits**: The `post-commit` hook (`.git/hooks/post-commit`) runs `cargo test --all-features` (line 29), which is slow and inappropriate for a post-commit hook. It also amends commits silently, which can break interactive workflows and rebase operations.

5. **`exitcode` crate unused**: Listed in `Cargo.toml` dependencies but the CLI defines its own `ExitCode` enum. This adds a needless compile-time dependency.

6. **CLI deps are unconditional**: `clap`, `clap_complete`, `anyhow`, `colored`, `exitcode` are always-on dependencies even for library-only users, inflating compile times.

## Decision Drivers
- LOC enforcement must be consistent across CI, local, and all source directories
- Pre-commit hooks prevent broken commits from reaching CI
- Dependency hygiene keeps compile times fast for library consumers

## Decision Outcome
Fix all six issues incrementally.

### Implementation
1. Update LOC gate in `scripts/validate.sh` and `.github/workflows/ci.yml` to use `find src -name '*.rs'` instead of `src/*.rs`
2. Create `scripts/pre-commit.sh` that runs `cargo fmt --check` and LOC gate (fast checks only)
3. Install pre-commit hook via `scripts/setup-hooks.sh`
4. Align clippy flags: use `cargo clippy --all-targets --all-features -- -D warnings` everywhere
5. Remove `exitcode` from `Cargo.toml` dependencies
6. Gate CLI dependencies behind `[features] cli = [...]` or move them to `[target.'cfg(not(wasm32))'.dependencies]`
7. Fix post-commit hook: remove `cargo test` call and avoid silent commit amending

### Positive Consequences
- Complete LOC enforcement across all 24 source files
- Fast pre-commit feedback loop
- Consistent CI/local behavior
- Faster compilation for library-only users

### Negative Consequences
- Pre-commit hook adds ~2s to each commit (fmt + LOC check only)
- Moving CLI deps to target-specific requires users building CLI to be on the native target (already true)
