# ADR-0036: CI/DX Hardening

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

CI and DX issues identified:
- LOC gate not recursive (misses src/cli/)
- No pre-commit hooks
- Clippy flags inconsistent between CI and local
- Post-commit hook runs tests (too slow)
- Unused exitcode crate

## Decision

Implement **CI/DX hardening**.

**Deliverables:**
- LOC gate: find src -name '*.rs' (recursive)
- Pre-commit: fmt --check + LOC gate (fast)
- Clippy flags: --all-targets --all-features -- -D warnings (consistent)
- Post-commit: remove test run, keep diagram auto-update
- Remove exitcode crate (use local ExitCode)

## Consequences

### Positive
- LOC gate covers all files
- Pre-commit catches issues early
- Consistent clippy results
- Fast commit workflow
- Cleaner dependencies

### Negative
- Pre-commit may slow commits
- LOC gate requires file splitting
- Post-commit less thorough

## Implementation

- Files: scripts/validate.sh, scripts/pre-commit.sh, .github/workflows/ci.yml
- Hooks: scripts/setup-hooks.sh installer

## Sources

- ACTIONS.md lines 1361-1440 (Phase 22 actions)
- ADR_REGISTRY.md: CI/DX Hardening details
- scripts/validate.sh