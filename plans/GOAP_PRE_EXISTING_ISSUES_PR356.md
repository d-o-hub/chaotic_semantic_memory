# GOAP: Pre-existing Issues Followup — PR #356 Codacy Remediation

> Tracking pre-existing issues discovered during Codacy remediation on PR
> [#356](https://github.com/d-o-hub/chaotic_semantic_memory/pull/356).
> Analysis date: 2026-06-09. Companion decision record:
> [ADR-0088](adr/0088-pre-existing-issues-pr356-codacy-remediation.md).

## 1. Goal State

```yaml
goal_state:
  mutation_test_passing: true           # BM25 boundary test added
  build_cli_all_platforms_green: true   # All Build CLI jobs passing
  action_sha_full_restored: true        # Full SHAs in workflow files
  codacy_warnings_resolved: true        # All unsafe/dead-code findings fixed
  pre_existing_issues_documented: true  # ADR-0088 registered
```

## 2. Current State (observed)

```yaml
world_state:
  # Codacy fixes (completed in this PR)
  codacy_unsafe_safety_comments_added: true     # embedding/mod.rs
  codacy_unused_import_removed: true            # bundle_simd.rs
  codacy_redundant_allow_removed: true          # hyperdim.rs
  # New fixes (completed 2026-06-09)
  bundle_simd_unused_simd_import_removed: true  # _mm256_set1_epi32 in update_counts_simd_avx2
  install_action_sha_restored: true             # Full SHA in ci.yml, pre-release-gate.yml
  hyperdim_data_moved_to_cfg_blocks: true       # Fixed aarch64 unused variable warning
  # Pre-existing issues (not fixed - followup on main)
  mutation_test_bm25_boundary_missed: true      # bm25.rs:272
  # PR status
  pr_356_codacy_green: true
  pr_356_lint_green: true
  pr_356_test_green: true
  pr_356_mcp_feature_green: true
  pr_356_mutation_test_red: true               # Pre-existing - followup on main
  pr_356_build_cli_all_green: true             # All platforms passing
  pr_356_merged: true                          # Merged despite mutation-test
```

## 3. Root-Cause Analysis

### Issue A: Mutation-test Failure (Pre-existing)

The mutation-test job found a missed mutation in `src/retrieval/bm25.rs:272`:

```
MISSED  src/retrieval/bm25.rs:272:33: replace > with >= in Bm25Index::search
```

This mutation survived because no test case exercises the exact boundary where
`score == threshold`. The current tests only check scores clearly above or
below the threshold.

**Impact**: mutation-test CI job fails on every run.

### Issue B: Build CLI aarch64 Compile Failure (Fixed)

The `update_counts_simd_avx2` function in `crates/csm-core/src/bundle_simd.rs`
imported `_mm256_set1_epi32` but never used it (scalar loop body replaced SIMD).
This caused a compile error on aarch64 where the x86_64 intrinsic is unavailable.

**Impact**: Build CLI (macos-arm64) failed to compile.
**Status**: FIXED — Removed unused import and variable.

### Issue C: Truncated Action SHA (Fixed)

Both `.github/workflows/ci.yml` and `.github/workflows/pre-release-gate.yml`
had a truncated commit SHA for `taiki-e/install-action`:

```yaml
# WRONG (truncated SHA):
uses: taiki-e/install-action@0631aa6  # v2.81.8

# CORRECT (full SHA):
uses: taiki-e/install-action@0631aa6515c7d545823c67cfae7ef4fc7f490154  # v2.81.8
```

**Impact**: mutation-test CI job may fail pinned-action validation.
**Status**: FIXED — Restored full SHA in both workflow files.

## 4. Action Plan (ordered, with preconditions/effects/costs)

```yaml
actions:
  # ═══════════════════════════════════════════════════════
  # COMPLETED: Codacy Fixes (in this PR)
  # ═══════════════════════════════════════════════════════
  - name: add_safety_comments_embedding_tests
    preconditions: []
    effects:
      codacy_unsafe_safety_comments_added: true
    cost: 1
    status: complete
    file: src/embedding/mod.rs
    description: |
      Add // SAFETY: comments to unsafe env var usage in embedding tests.
      Documents why unsafe is sound (single-threaded test, no concurrent readers).

  - name: remove_unused_rngext_import
    preconditions: []
    effects:
      codacy_unused_import_removed: true
    cost: 1
    status: complete
    file: crates/csm-core/src/bundle_simd.rs
    description: |
      Remove unused `use rand::RngExt;` import. random_range() is from rand::Rng.

  - name: remove_redundant_allow_attributes
    preconditions: []
    effects:
      codacy_redundant_allow_removed: true
    cost: 1
    status: complete
    file: crates/csm-core/src/hyperdim.rs
    description: |
      Remove #[allow(unused_mut, unused_variables)] where mut is genuinely
      required for rng.fill(&mut data) and data.as_mut_ptr().

  # ═══════════════════════════════════════════════════════
  # COMPLETED: Build CLI & Workflow Fixes (2026-06-09)
  # ═══════════════════════════════════════════════════════
  - name: remove_unused_simd_import
    preconditions: []
    effects:
      bundle_simd_unused_simd_import_removed: true
    cost: 1
    status: complete
    file: crates/csm-core/src/bundle_simd.rs
    description: |
      Remove unused `use std::arch::x86_64::_mm256_set1_epi32;` and
      `let _sign_vec = _mm256_set1_epi32(sign);` from update_counts_simd_avx2.
      Function uses scalar loop, not SIMD intrinsics.

  - name: restore_full_install_action_sha
    preconditions: []
    effects:
      install_action_sha_restored: true
    cost: 1
    status: complete
    file: .github/workflows/ci.yml, .github/workflows/pre-release-gate.yml
    description: |
      Restore full commit SHA for taiki-e/install-action from truncated
      `0631aa6` to full `0631aa6515c7d545823c67cfae7ef4fc7f490154` (v2.81.8).

  # ═══════════════════════════════════════════════════════
  # FOLLOWUP: Pre-existing Issues
  # ═══════════════════════════════════════════════════════
  - name: add_bm25_boundary_test
    preconditions:
      codacy_warnings_resolved: true
    effects:
      mutation_test_passing: true
    cost: 3
    status: queued
    file: src/retrieval/bm25/tests.rs
    description: |
      Add a test case where BM25 score exactly equals the threshold to verify
      strict inequality (> vs >=) is intentional. This kills the mutation.

  - name: investigate_build_cli_timeouts
    preconditions: []
    effects:
      build_cli_timeout_investigated: true
    cost: 2
    status: queued
    description: |
      Investigate why Build CLI jobs timeout across all platforms. Check:
      - Workflow timeout configuration
      - Cross-compilation toolchain setup time
      - Caching effectiveness
      - Runner resource availability

  - name: consider_temp_env_crate
    preconditions: []
    effects:
      test_env_management_improved: true
    cost: 2
    status: deferred
    description: |
      Evaluate using `temp_env` crate for safer test environment variable
      management instead of raw unsafe set_var/remove_var. Currently safe
      but could reduce boilerplate.

  - name: register_adr_0088
    preconditions: []
    effects:
      adr_0088_registered: true
    cost: 1
    status: queued
    file: plans/ADR_REGISTRY.md
    description: |
      Add ADR-0088 to the ADR registry table and run check-adr-parity.sh.
```

## 5. Validation Gates (must pass before claiming done)

```bash
# For BM25 boundary test
cargo test --lib retrieval::bm25
cargo mutants --filter bm25.rs --check  # Verify mutation killed

# For Build CLI investigation
gh run list --workflow=ci.yml --branch <branch> --limit 3
gh run view <run-id> --json jobs --jq '.jobs[] | select(.name | startswith("Build CLI"))'

# For ADR registration
./scripts/check-adr-parity.sh
```

## 6. Completion Status

**PR #356 Merged**: 2026-06-09 (merged despite mutation-test failure)

**Codacy Fixes** (commit `5876bc7`):

- Added SAFETY comments to unsafe env var usage in embedding tests
- Removed unused `use rand::RngExt;` from bundle_simd.rs
- Removed redundant `#[allow(unused_mut, unused_variables)]` from hyperdim.rs
- Restored action versions to latest pinned SHAs

**Build CLI & Workflow Fixes** (commit `a1b573a`):

- Removed unused `_mm256_set1_epi32` import from `update_counts_simd_avx2`
- Restored full SHA for `taiki-e/install-action` in ci.yml and pre-release-gate.yml

**aarch64 Build Fix** (commit `b238d8e`):

- Moved `let mut data = [0u128; 80]` inside `#[cfg]` blocks to fix unused variable warning on aarch64

**Pre-existing Issues** — Followup on main:

- mutation-test: BM25 boundary test needed (cost: 3)
- temp_env: Deferred evaluation (cost: 2)

## 7. Notes / Out-of-Scope

- PR #356 merged with mutation-test failure as pre-existing issue
- All other CI checks pass: lint, test, mcp-feature, wasm, Build CLI (all platforms)
- The Codacy fixes are minimal and targeted — no behavioral changes
- The mutation-test failure predates PR #356 and is not a regression
- The `#[allow(dead_code)]` on `bundle_word_scalar` in hyperdim.rs is
  legitimate (function is conditionally compiled via cfg attributes)
- hyperdim_tests.rs imports are correct via the `include!` macro context
  with `use super::*` bringing in `HVec10240` from the parent test module
