# [ADR-0037] Rust Best Practices: #[must_use], Unsafe Docs, JSON Safety

## Status
Proposed

## Context and Problem Statement
A Rust conventions audit identified several patterns that deviate from idiomatic Rust and Clippy best practices:

1. **Missing `#[must_use]` on public constructors/factory methods**: `HVec10240::zero()`, `random()`, `sparse()`, `bundle()`, `bind()`, `permute()`, `cosine_similarity()`, `hamming_distance()`, `Singularity::new()`, `Reservoir::new()`, `to_hypervector()`, `ChaoticSemanticFramework::builder()` all return values that should not be silently discarded.

2. **Minimal unsafe documentation**: SIMD intrinsics in `src/hyperdim.rs` (lines 30-36, 57-62) have SAFETY comments but do not explicitly document alignment and bounds requirements for `cast::<__m128i>()` on `u128` pointers.

3. **Unjustified clippy suppressions**: `#![allow(clippy::needless_range_loop)]` is applied file-wide in `hyperdim.rs` rather than targeted at specific loops, and `#[allow(unreachable_code)]` blocks (lines 193, 224) exist as artifacts of cfg-branch platform gating that could be restructured.

4. **CLI JSON output uses format! instead of serde_json**: Commands in `inject.rs`, `associate.rs`, `export.rs`, `import.rs`, and `mod.rs` construct JSON strings via `format!`/`println!` macros, which doesn't escape special characters in concept IDs or file paths.

5. **`serde_json::to_string().unwrap()` in probe.rs**: Lines 57 and 130 use `.unwrap()` on JSON serialization which can technically fail on non-UTF-8 data.

## Decision Drivers
- `#[must_use]` prevents silent logic errors (missing result checks)
- Proper unsafe documentation is a Rust community norm and aids audit
- JSON escaping prevents downstream tool breakage
- Targeted clippy allows are easier to review and remove

## Decision Outcome
Apply all five improvements as a single "Rust conventions alignment" pass.

### Implementation
1. Add `#[must_use]` to all public constructors and methods that return meaningful values
2. Expand SAFETY comments on SIMD blocks to cover alignment guarantees (`u128` is 16-byte aligned, matching `__m128i` requirements)
3. Replace file-wide `#![allow(clippy::needless_range_loop)]` with per-loop `#[allow(...)]` on the specific loops that need it; restructure cfg-branch returns to eliminate `#[allow(unreachable_code)]`
4. Replace all `format!`-based JSON output in CLI with `serde_json::json!` macro
5. Replace `.unwrap()` on `serde_json::to_string()` with `.context("JSON serialization failed")?`

### Positive Consequences
- Clippy clean with zero global suppressions
- Safe JSON output for any concept ID content
- Better unsafe documentation for security audits
- Compiler warns on discarded return values

### Negative Consequences
- Minor code churn (~50 lines across 8 files)
