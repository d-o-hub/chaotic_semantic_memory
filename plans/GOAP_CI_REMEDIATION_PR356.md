# GOAP: CI Failure Remediation — PR #356

> Workspace-split PR ["Partial Split of Monolithic Crate into Workspace
> Members"](https://github.com/d-o-hub/chaotic_semantic_memory/pull/356).
> Analysis date: 2026-06-09. Head commit analyzed: `0e29159`.
> Companion decision record: [ADR-0087](adr/0087-ci-failure-remediation-pr356.md).

## 1. Goal State

```yaml
goal_state:
  pr_356_test_job_passing: true        # CI "test" job green
  pr_356_mcp_feature_job_passing: true # CI "mcp-feature" job green
  pr_356_lint_job_passing: true        # CI "lint" job green (fmt + clippy)
  pr_356_all_checks_green: true
```

## 2. Current State (observed)

```yaml
world_state:
  pr_356_open: true
  pr_356_mergeable: true               # no merge conflicts with main
  pr_356_changed_files: 88
  # CI status on head 0e29159 (run 27166273177)
  check_test: FAILURE
  check_mcp_feature: FAILURE
  check_lint: FAILURE
  check_wasm: SUCCESS
  check_duckdb_companion: SUCCESS
  check_codeql_rust: SUCCESS
  check_codacy: SUCCESS
  check_sonarcloud: SUCCESS
  check_version_integrity: SUCCESS
  check_benchmark_small: SUCCESS
  # Build CLI / mutation-test SKIPPED (gated behind earlier jobs)
```

## 3. Root-Cause Analysis (single source for all 3 failures)

All three red checks originate from **one** newly added test block in
[`src/cli/commands/mod.rs`](../src/cli/commands/mod.rs) (added in `0e29159`,
function `test_create_framework_advanced_config`, lines ~195–234).

```diagram
╭───────────────────────────────────────────────────────────╮
│ src/cli/commands/mod.rs  tests::test_create_framework_...   │
╰───────────────┬───────────────────────────┬───────────────╯
                │                           │
   line 231: v_true.cosine_similarity()   line 224: assert_ne!(...) too long
   on a Vec<f32>  → E0599                   → rustfmt diff
                │                           │
        ╭───────┴────────╮          ╭───────┴───────╮
        ▼                ▼          ▼               
   ┌─────────┐   ┌──────────────┐  ┌──────────┐
   │  test   │   │ mcp-feature  │  │   lint   │
   │ FAILURE │   │   FAILURE    │  │ FAILURE  │
   └─────────┘   └──────────────┘  └──────────┘
```

### Failure A — `test` and `mcp-feature` (compile error E0599)

```
error[E0599]: no method named `cosine_similarity` found for struct
              `std::vec::Vec<f32>` in the current scope
  --> src/cli/commands/mod.rs:231:26
```

`EmbeddingProvider::embed()` returns `Result<Vec<f32>>` (see
[`src/embedding/mod.rs:51`](../src/embedding/mod.rs)). `cosine_similarity` is a
method on the `HVec10240` struct in `csm-core`
([`crates/csm-core/src/hyperdim.rs:319`](../crates/csm-core/src/hyperdim.rs)),
**not** on `Vec<f32>`. The test calls it on the raw vector, which does not
compile. Because the lib test target fails to compile, both the `test` job and
the `mcp-feature` job (which also builds tests) abort with exit 101.

### Failure B — `lint` (rustfmt diff)

```
##[warning]Diff in .../src/cli/commands/mod.rs:224:
-        assert_ne!(v_true, v_false, "Vectors should differ based on code_aware config");
+            "Vectors should differ based on code_aware config"
```

`cargo fmt --all -- --check` wants the long `assert_ne!` line wrapped. The job
runs `fmt` before `clippy`, so it exits 1 on the unformatted line. (clippy could
not run; it would also have failed to compile on Failure A.)

## 4. Action Plan (ordered, with preconditions/effects/costs)

```yaml
actions:
  - name: fix_cosine_similarity_on_vec_f32
    preconditions:
      pr_356_open: true
    effects:
      check_test: SUCCESS
      check_mcp_feature: SUCCESS
    cost: 2
    file: src/cli/commands/mod.rs
    description: |
      Replace `v_true.cosine_similarity(&v_split)` with an inline cosine
      computation over Vec<f32>, since embed() returns Vec<f32> and there is no
      cosine helper on that type. See ADR-0087 §Decision for the exact snippet.

  - name: rustfmt_long_assert_lines
    preconditions:
      fix_cosine_similarity_on_vec_f32: done
    effects:
      check_lint: SUCCESS
    cost: 1
    file: src/cli/commands/mod.rs
    description: |
      Run `cargo fmt --all` to wrap the over-length assert_ne!/assert! lines.

  - name: revalidate_locally
    preconditions:
      fix_cosine_similarity_on_vec_f32: done
      rustfmt_long_assert_lines: done
    effects:
      local_gates_green: true
    cost: 2
    description: |
      cargo fmt --all -- --check
      cargo clippy --all-targets --all-features -- -D warnings
      cargo test --lib cli::commands
      cargo test --features mcp --no-run

  - name: push_and_confirm_ci
    preconditions:
      local_gates_green: true
    effects:
      pr_356_all_checks_green: true
    cost: 1
    description: |
      Push to the PR branch and confirm test/mcp-feature/lint go green via
      `gh pr checks 356 --watch`.
```

## 5. Validation Gates (must pass before claiming done)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib
cargo test --features mcp --no-run
gh pr checks 356 --watch
```

## 6. Completion Status

**Fixed in commit `10e63ae`** (2026-06-09):

- Replaced `v_true.cosine_similarity(&v_split)` with inline dot-product
  computation (method only exists on `HVec10240`, not `Vec<f32>`)
- Wrapped long `assert_ne!`/`assert!` lines for rustfmt compliance
- All CI checks green: lint, test, mcp-feature, wasm, Codacy, CodeQL,
  SonarCloud, benchmark-small, Build CLI (all platforms)

## 7. Notes / Out-of-Scope

- The structural workspace-split work (`crates/csm-core`, observability OTLP/Prom
  additions, `src/hyperdim_simd.rs` removal) is **not** the cause of any red
  check and is out of scope for this remediation. The PR description itself flags
  the split as still in progress; this GOAP only restores CI to green for the
  current head.
- Mutation-test and Build-CLI jobs are SKIPPED, not failing; they are gated on
  the upstream jobs and will run once test/lint pass.
```

