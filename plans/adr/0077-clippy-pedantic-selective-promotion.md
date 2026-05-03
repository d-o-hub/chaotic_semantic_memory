# ADR-0077: Clippy Pedantic — Selective Promotion to `warn`

## Status

Phase A+B Implemented (2026-05-01) — Phase C deferred

## Context and Problem Statement

Current lint configuration in `Cargo.toml` and `.clippy.toml`:

```toml
[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "allow", priority = -1 }   # blanket allow
nursery = { level = "allow", priority = -1 }    # blanket allow
```

`cargo clippy --all-features --all-targets -- -D warnings` is green (verified 2026-04-30).

Probing with `-W clippy::pedantic -W clippy::nursery` surfaces **936 warnings**. A blanket-allow on pedantic hides several **correctness-relevant** lints:

| Pedantic lint | Hits | Why it matters |
|---|---|---|
| `clippy::float_cmp` | 44 | Strict `==` on f32/f64 — almost always a bug in similarity / spectral radius code |
| `clippy::significant_drop_tightening` | 21 | Lock held longer than needed → contention in async hot paths (we have RwLock everywhere) |
| `clippy::cast_precision_loss` | 13 | `usize as f32` losing bits in our 10240-dim math |
| `clippy::missing_errors_doc` | 121 | Public Result-returning APIs with no `# Errors` section — poor library DX |
| `clippy::must_use_candidate` | 51 | Caller silently drops a Result/Option → silent bug |
| `clippy::missing_const_for_fn` | 25 | Free perf wins on builders / pure helpers |
| `clippy::redundant_clone` | 7 | Free perf wins |

The remaining ~650 hits are cosmetic (`uninlined_format_args`, `doc_markdown`, `redundant_else`, `semicolon_if_nothing_returned`, etc.) and not worth chasing.

## Decision Drivers

- Don't break the existing green CI by flipping a blanket switch
- Promote only **correctness** and **library DX** lints, not cosmetic ones
- Keep `pedantic = allow` as the default; opt **specific** lints to `warn`
- Each promotion must be independently fixable in ≤ 2 PRs

## Considered Options

1. **Selective promotion** of 5 high-value pedantic lints to `warn`
2. Flip pedantic to `warn` globally, then allow noisy ones individually
3. Stay with current blanket allow

## Decision Outcome

Chosen: **Option 1**. Lowest risk; isolates correctness signal from cosmetic noise.

## Implementation

### Phase A — Add lints (one PR, no source changes)

Update `Cargo.toml`:

```toml
[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "allow", priority = -1 }
nursery = { level = "allow", priority = -1 }

# Promoted from pedantic — correctness signal
float_cmp = "warn"
significant_drop_tightening = "warn"
cast_precision_loss = "warn"
cast_possible_truncation = "warn"
redundant_clone = "warn"

# Promoted from pedantic — library DX (allow gradually, then warn)
missing_errors_doc = "allow"     # Phase B will flip to warn
must_use_candidate = "allow"     # Phase B will flip to warn

# Promoted from nursery — free perf wins
missing_const_for_fn = "warn"

# Existing allows preserved
too_many_arguments = "allow"
type_complexity = "allow"
needless_range_loop = "allow"
unwrap_used = "allow"
expect_used = "allow"
panic = "allow"
print_stdout = "allow"
print_stderr = "allow"
many_single_char_names = "allow"
cognitive_complexity = "allow"
```

Run `cargo clippy --all-features --all-targets -- -D warnings`. Expected: **44 + 21 + 13 + 7 + 25 ≈ 110 warnings** to address.

### Phase B — Fix in 5 themed PRs

| PR | Theme | Lint(s) | File scope |
|---|---|---|---|
| 1 | Float equality | `float_cmp` | `reservoir.rs`, `singularity_*.rs`, `hyperdim*.rs`, tests |
| 2 | Lock tightening | `significant_drop_tightening` | `framework*.rs`, `singularity_cache.rs` |
| 3 | Cast safety | `cast_precision_loss`, `cast_possible_truncation` | `hyperdim.rs`, `reservoir.rs`, `retrieval/bm25.rs` |
| 4 | Const fns | `missing_const_for_fn` | builders, pure helpers across all modules |
| 5 | Redundant clones | `redundant_clone` | small surface, mostly tests |

Each PR ≤ 250 LOC delta, tests must remain green, no behavior change.

### Phase C — Library DX (after Phase B settles)

- Flip `missing_errors_doc = "warn"` in `Cargo.toml`
- Add `# Errors` sections to all 121 affected fns in batches by module
- Flip `must_use_candidate = "warn"`, audit each candidate

### Verification

```bash
# Before each PR
export CARGO_TERM_PROGRESS_WHEN=never
cargo clippy --all-features --all-targets -- -D warnings 2>&1 | tail -5

# Smoke
cargo test --all-features --quiet
```

### CI

`.github/workflows/ci.yml` already runs `cargo clippy --all-targets --all-features -- -D warnings`. No CI changes needed — promoted lints flow through automatically.

## Pros and Cons

### Pros
- Catches real correctness bugs (float equality, lock contention, cast loss)
- Small, reviewable PRs
- Preserves green-CI invariant at every step
- Improves library DX without flipping cosmetic lints

### Cons
- Five PRs of cleanup work (~110 sites)
- Some `float_cmp` hits are intentional (exact zero) — need `#[allow]` per-site or `approx_eq!` macro
- `missing_const_for_fn` may surface trait-coherence issues that block promotion

## Acceptance Criteria

- [x] Phase A `Cargo.toml` change merged
- [x] Phase B PRs 1-5 merged, all sites fixed or annotated
- [x] `cargo clippy --all-features --all-targets -- -D warnings` green throughout
- [x] No test regressions
- [ ] Phase C (`missing_errors_doc`, `must_use_candidate`) tracked as Wave 23 follow-up
