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
