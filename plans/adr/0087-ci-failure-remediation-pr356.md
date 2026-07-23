# ADR-0087: CI Failure Remediation for PR #356 (Workspace Split)

## Status

Proposed

## Context and Problem Statement

PR [#356](https://github.com/d-o-hub/chaotic_semantic_memory/pull/356) ("Partial
Split of Monolithic Crate into Workspace Members", head commit `0e29159`) has
three failing CI checks on run `27166273177`:

- `test` — FAILURE
- `mcp-feature` — FAILURE
- `lint` — FAILURE

All other checks (`wasm`, `Test DuckDB Companion`, CodeQL, Codacy, SonarCloud,
Version Integrity, `benchmark-small`) pass. `Build CLI` and `mutation-test` are
SKIPPED because they are gated behind the failing jobs.

Investigation shows **all three failures share a single root cause**: a new unit
test `test_create_framework_advanced_config` added to
[`src/cli/commands/mod.rs`](../../src/cli/commands/mod.rs) in commit `0e29159`.

1. **Compile error (E0599)** — drives both `test` and `mcp-feature`:

   ```
   error[E0599]: no method named `cosine_similarity` found for struct
                 `std::vec::Vec<f32>`
     --> src/cli/commands/mod.rs:231:26
   ```

   `EmbeddingProvider::embed()` returns `Result<Vec<f32>>`
   ([`src/embedding/mod.rs:51`](../../src/embedding/mod.rs)). `cosine_similarity`
   exists only on the `HVec10240` struct in `csm-core-lib`
   ([`crates/csm-core-lib/src/hyperdim.rs:319`](../../crates/csm-core-lib/src/hyperdim.rs)),
   not on `Vec<f32>`. The lib-test target therefore fails to compile, aborting
   both jobs.

2. **rustfmt diff** — drives `lint`:

   The `assert_ne!(v_true, v_false, "Vectors should differ based on code_aware
   config");` line (and the `assert!(sim > 0.5, ...)` line) exceed the line
   width, so `cargo fmt --all -- --check` reports a diff and the `lint` job exits
   1 before clippy even runs.

This is a test-only defect in otherwise-healthy work; the workspace-split itself
compiles and the WASM/DuckDB/static-analysis gates are green.

## Decision Drivers

- Restore PR #356 to all-green CI with the **smallest correct change**.
- Keep the test's intent (verify code-aware HDC encoding preserves similarity)
  intact rather than deleting it.
- Avoid introducing a new public API just to satisfy a test.

## Considered Options

### Option 1: Inline cosine computation over `Vec<f32>` + run `cargo fmt`

Replace the invalid method call with a direct dot/norm calculation on the two
`Vec<f32>` vectors, and let `cargo fmt --all` wrap the long assert lines.

- Good: minimal, test-local, no API surface change.
- Good: fixes all three jobs from one edit.
- Neutral: a few lines of arithmetic in the test.

### Option 2: Add a `cosine_similarity` helper for `Vec<f32>` / slices

Introduce a crate utility usable from the test.

- Good: reusable.
- Bad: new public/exported surface for a one-off test assertion; over-engineering
  per the repo's pragmatism rules; needs its own tests and doc.

### Option 3: Convert `Vec<f32>` to `HVec10240` and call its method

- Bad: HDC text embeddings are dense projected vectors, not native bit
  hypervectors; the conversion semantics are wrong for this assertion and would
  test the wrong thing.

## Decision Outcome

Chosen option: **Option 1 — inline cosine over `Vec<f32>` plus `cargo fmt`**.

Recommended edit (test body, replacing the failing lines):

```rust
// Verify code-aware behavior: "my_function_name" should be similar to
// "my function name".
let v_split = fw_true.embedding_provider.embed("my function name").await.unwrap();
let dot: f32 = v_true.iter().zip(&v_split).map(|(a, b)| a * b).sum();
let norm_a = v_true.iter().map(|x| x * x).sum::<f32>().sqrt();
let norm_b = v_split.iter().map(|x| x * x).sum::<f32>().sqrt();
let sim = if norm_a == 0.0 || norm_b == 0.0 {
    0.0
} else {
    dot / (norm_a * norm_b)
};
assert!(
    sim > 0.5,
    "Code-aware encoding should preserve similarity after splitting, got {sim}"
);
```

Then run `cargo fmt --all` to wrap the over-length `assert_ne!` line.

### Positive Consequences

- `test`, `mcp-feature`, and `lint` all return to green from one focused edit.
- No new API surface; existing crate boundaries (post-split) are respected.
- Test intent (code-aware similarity preservation) is preserved.

### Negative Consequences

- The cosine math is duplicated inline in the test rather than shared; acceptable
  for a single test assertion (some duplication beats premature abstraction).

## Follow-up Actions

- [ ] Apply the inline cosine fix in `src/cli/commands/mod.rs`.
- [ ] Run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --lib` and `cargo test --features mcp --no-run`.
- [ ] Push to the PR branch; confirm green via `gh pr checks 356 --watch`.
- [ ] Register ADR-0087 in `plans/ADR_REGISTRY.md` and run
      `scripts/check-adr-parity.sh`.
