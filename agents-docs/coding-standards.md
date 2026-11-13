# Coding Standards & DeepSource Parity

These patterns mirror DeepSource's Rust analyzer rules and repository clippy lints. Violations block CI.

## 1. Construct Directly in `Default::default()`
```rust
// ✅ CORRECT: default() constructs the struct directly; new() delegates to default()
impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self {
            config: FrameworkConfig::default(),
            db_path: None,
        }
    }
}

impl FrameworkBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

// ❌ WRONG: default() calling Self::new() triggers DeepSource BUG_RISK
impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self::new()  // DeepSource: "Found call returning Self in default()"
    }
}
```

## 2. Use `.map_or()` / `.is_some_and()` instead of `.map().unwrap_or()`
```rust
// ✅ CORRECT
concepts.get(id).is_some_and(|c| filter.matches(&c.metadata))
value.map_or_else(|| default(), |s| s.to_string())

// ❌ WRONG: triggers DeepSource ANTI_PATTERN + clippy::map_unwrap_or
concepts.get(id).map(|c| filter.matches(&c.metadata)).unwrap_or(false)
value.map(|s| s.to_string()).unwrap_or_else(|| default())
```

## 3. Clippy Lints Enforcing These
| Pattern | Clippy Lint | Active? |
|---------|-------------|---------|
| `.map(f).unwrap_or(g)` | `clippy::map_unwrap_or` (pedantic) | ✅ Promoted to `warn` in Cargo.toml |
| `.map_or(false, f)` | `clippy::unnecessary_map_or` (in `all`) | ✅ Implied by `-D warnings` |
| `Self::new()` in `default()` | No clippy equivalent | 📋 Documented above |
| `unwrap()`/`expect()`/`panic!()` in library | `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic` | ✅ `warn` in workspace lints |

## 4. Lint Policy: unwrap/expect/panic
- **Workspace lints** (`Cargo.toml`): `unwrap_used = "warn"`, `expect_used = "warn"`, `panic = "warn"`
- **Test exemption** (`.clippy.toml`): `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true`, `allow-panic-in-tests = true`
- **Production allows**: Use `#[allow(clippy::expect_used)]` with a justification comment for infallible operations (lock acquisition, static construction).
- **CI enforcement**: `cargo clippy -- -D warnings` promotes all warnings to errors.

## 5. cfg-Partitioned Module Graph
`src/lib.rs` splits the framework by target: `#[cfg(not(target_arch = "wasm32"))]` on the
file-I/O modules (`framework_ops`, `framework_ops_import` — they use `tokio::fs`) and
`#[cfg(target_arch = "wasm32")]` on the bindings (`wasm`, `wasm_ext`, `wasm_graph_rag`).

- A native build never sees `src/wasm.rs`; a WASM build never sees the file-I/O modules. Host
  clippy, `cargo doc`, the test suite and the LOC gate therefore **cannot** detect a call from one
  half into the other.
- Shared logic goes in a module compiled for both targets (see `framework_import_apply.rs`: the
  filesystem-free validate-then-install path). Do not fork a weaker second path per target — the
  copies drift, and the drift is invisible on whichever target the tests do not run.
- `#[cfg(target_arch = "wasm32")] use ...` is dead code on the host: clippy will not tell you.
  Delete it when the copied call site is gone.
- Gate: `cargo check --target wasm32-unknown-unknown -p csm-wasm` (last stage of
  `scripts/validate.sh`, mirrored by the ci.yml `WASM` job).

## 6. Manifest Hygiene: Cargo Lints Are Denied on the `stable` Canary

The ci.yml `stable-toolchain` job installs `toolchain: stable` and
`actions-rust-lang/setup-rust-toolchain` exports `CARGO_BUILD_WARNINGS=deny`, so a *manifest* lint
that the pinned 1.88 toolchain only warns about becomes a hard error the day stable adds it
(1.100.0: `cargo::unused_dependencies`, `cargo::redundant_homepage`; run `43403864562` failed with
exit 101 on a PR whose diff was unrelated).

- Delete the dead weight; do not allow the lint. `[lints.cargo]` is accepted by 1.100 but prints
  "unused manifest key" on 1.88, and an allow silences the whole class — including the case below.
- `unused_dependencies` is **per target**: a host build flags deps that only a
  `wasm32-unknown-unknown` build consumes (csm-wasm's `wasm-bindgen`/`js-sys`/`web-sys`/`getrandom`
  sit next to a `lib.rs` that only re-exports). Verify with the other target's gate
  (`cargo check --target wasm32-unknown-unknown -p csm-wasm`) before deleting anything.
- `[workspace.package]` keys must be inherited by someone (`rust-version.workspace = true`); a key
  no member inherits is dead, and `homepage` equal to `repository` is redundant.
- Reproduce locally before editing:
  `rustup toolchain install 1.100.0 && CARGO_BUILD_WARNINGS=deny cargo +1.100.0 check --workspace --all-targets`.
