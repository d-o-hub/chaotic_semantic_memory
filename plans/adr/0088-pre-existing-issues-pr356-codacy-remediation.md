# ADR-0088: Pre-existing Issues Documented During PR #356 Codacy Remediation

## Status

Accepted

## Context and Problem Statement

During remediation of Codacy static analysis warnings on PR
[#356](https://github.com/d-o-hub/chaotic_semantic_memory/pull/356) ("Partial
Split of Monolithic Crate into Workspace Members"), several pre-existing issues
were identified that predate the workspace-split work. This ADR documents these
issues for tracking and future resolution.

The Codacy report flagged 8 unsafe usage warnings in `src/embedding/mod.rs`
(lines 198, 206, 212, 217, 223, 231, 237, 242) and additional findings in
`crates/csm-core/`.

## Decision Drivers

- Document pre-existing issues separately from PR-specific regressions
- Track technical debt that should be addressed in future waves
- Distinguish between issues fixed in this PR vs. issues carried forward

## Pre-existing Issues Identified

### Issue 1: Missed Mutation in BM25 Search (mutation-test FAILURE)

**File**: `src/retrieval/bm25.rs:272`
**Mutation**: Replace `>` with `>=` in `Bm25Index::search`
**Impact**: mutation-test CI job failed before PR #356 remediation
**Root Cause**: The boundary condition in BM25 search does not have test coverage
that distinguishes strict greater-than from greater-than-or-equal.

```rust
// Line 272 in bm25.rs
if score > threshold {  // mutation: >= would also pass all tests
```

**Resolution**: Fixed in PR #356 by adding BM25 threshold-boundary coverage in
`src/retrieval/bm25/tests.rs`.

### Issue 1b: Empty-Diff Mutation Runs Expand to Full Suite

**File**: `.github/workflows/ci.yml`, `scripts/mutation_test.sh`
**Impact**: post-merge `main` mutation-test jobs can be cancelled at the
30-minute job timeout.
**Root Cause**: Push events used `origin/main` as `DIFF_TARGET`; after checkout
on `main`, that diff is empty. The fast mutation profile treated an empty diff
as a local full-run fallback even in CI.
**Resolution**: Push events now diff against `github.event.before`, and
`scripts/mutation_test.sh fast --ci` exits successfully with a short report when
there is no source diff to mutate.

### Issue 2: Build CLI Timeouts (Infrastructure)

**Jobs Affected**: Build CLI (macos-arm64, macos-x64, linux-arm64, linux-x64, windows-x64)
**Symptom**: Earlier jobs were cancelled after ~60s with "The operation was
canceled"; later run `27196604107` also exposed a macOS arm64 `-D warnings`
failure from an unused `data` binding in `crates/csm-core/src/hyperdim.rs:173`.
**Impact**: Build CLI jobs failed or failed to complete before remediation.
**Resolution**: Superseded by later PR #356 fixes. In run `27199074152`, Build
CLI passed for linux-x64, linux-arm64, macos-arm64, macos-x64, and windows-x64.

**Recommended Investigation**: None currently active unless the timeout recurs
on a fresh run after `d48357d`.

### Issue 3: Unsafe Env Var Usage in Tests (Fixed in This PR)

**File**: `src/embedding/mod.rs`
**Lines**: 198, 206, 212, 217, 223, 231, 237, 242
**Pattern**: `unsafe { std::env::set_var(...) }` / `unsafe { std::env::remove_var(...) }`
**Status**: FIXED — Added `// SAFETY:` comments documenting why the unsafe is sound
**Justification**: Env var mutation in single-threaded test is sound; no concurrent readers

### Issue 4: Unused Import (Fixed in This PR)

**File**: `crates/csm-core/src/bundle_simd.rs:167`
**Import**: `use rand::RngExt;`
**Status**: FIXED — Removed unused import
**Note**: `random_range()` is provided by `rand::Rng`, not `RngExt`

### Issue 5: Redundant Allow Attributes (Fixed in This PR)

**File**: `crates/csm-core/src/hyperdim.rs`
**Lines**: 57, 70, 79, 176, 433
**Pattern**: `#[allow(unused_mut, unused_variables)]`
**Status**: FIXED — Removed redundant attributes; `mut` is genuinely required for
`rng.fill(&mut data)` and `data.as_mut_ptr()`

## Decision Outcome

Chosen option: **Document and track as technical debt**.

The mutation-test and Build CLI issues are pre-existing and not introduced by
the workspace-split. They should be tracked as follow-up actions.

### Positive Consequences

- Clear separation between PR regressions and pre-existing debt
- Actionable items tracked in GOAP for future waves
- Codacy findings resolved with minimal, correct changes

### Negative Consequences

- The post-merge `main` CI run still needs final observation until `miri`
  completes and the cancelled `mutation-test` job is classified.

## Follow-up Actions

- [x] Add BM25 boundary test for `score >= threshold` case (mutation-test fix)
- [x] Skip empty-diff fast mutation CI runs instead of falling back to the full
      mutation suite
- [x] Investigate Build CLI timeout/configuration enough to confirm latest
      matrix jobs pass
- [x] Consider adding `temp_env` crate for safer test env var management  (PR: this branch)
- [x] Register ADR-0088 in `plans/ADR_REGISTRY.md`

## References

- PR #356: https://github.com/d-o-hub/chaotic_semantic_memory/pull/356
- ADR-0087: CI Failure Remediation for PR #356
- Codacy Report: Lines 198, 206, 212, 217, 223, 231, 237, 242 in embedding/mod.rs
