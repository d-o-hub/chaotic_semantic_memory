# GOAP Plan: Clippy Lints Best Practices

**Date**: 2026-04-29
**Orchestrator**: goap_clippy_analysis_2026_04_29
**Baseline**: Clippy passes with `-D warnings`, 6 justified allows

---

## Current World State Summary

| Metric | Value |
|--------|-------|
| Clippy status | ✅ Passing (`-D warnings`) |
| File-wide allows | 0 (library code) |
| Per-item allows | Justified production allows only |
| `.clippy.toml` | ✅ Created (cognitive complexity, allow-*-in-tests) |
| `Cargo.toml` lints | ✅ Added (workspace lints) |
| Test exemptions | ✅ `.clippy.toml` allow-unwrap-in-tests/allow-expect-in-tests/allow-panic-in-tests |

### Current `#[allow(...)]` Usage

| Location | Allow | Justification | Valid |
|----------|-------|---------------|-------|
| `export_payload.rs:71,101,122,145` | `dead_code` | Serialization struct fields | ✅ Valid |
| `singularity_retrieval.rs:335` | `too_many_arguments` | Internal stats function (>7 args) | ✅ Valid (GOAP_STATE notes) |
| `framework_ops.rs:160` | `type_complexity` | Complex generic type | ✅ Valid |
| `hyperdim.rs:258` | `needless_range_loop` | Intentional range iteration | ✅ Valid |

---

## Recommended Clippy Best Practices

### A. Add `.clippy.toml` Configuration

Create `.clippy.toml` to configure lint behavior:

```toml
# .clippy.toml - Clippy lint configuration

# Cognitive complexity threshold for methods
cognitive-complexity-threshold = 25

# Maximum lines per function
max-lines-per-function = 100

# Maximum arguments per function (internal functions allowed)
# Note: #[allow(clippy::too_many_arguments)] used for internal hot paths
```

### B. Add `Cargo.toml` Lints Section (Rust 2024)

Since edition is 2024, add lints table:

```toml
# Cargo.toml - Add after [dependencies]

[lints.rust]
unsafe_code = "warn"
missing_docs = "allow"  # Not enforced (project uses selective docs)

[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "allow", priority = -1 }
nursery = { level = "allow", priority = -1 }

# Explicit lint preferences
too_many_arguments = "allow"  # Internal hot paths documented
type_complexity = "allow"     # Complex generics in framework
dead_code = "allow"           # Serialization struct fields
needless_range_loop = "allow" # Intentional range iteration

# Warn on these
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"

# Deny on these
print_stdout = "deny"
print_stderr = "deny"
```

### C. Update `.github/workflows/ci.yml`

Ensure CI uses consistent clippy flags:

```yaml
- name: Clippy
  run: cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

---

## GOAP Actions

### Target State
```yaml
clippy_config_file_created: true
cargo_lints_section_added: true
clippy_ci_flags_consistent: true
clippy_best_practices_enforced: true
```

### Ordered Actions

```yaml
actions:
  - name: create_clippy_config_file
    preconditions:
      tests_passing: true
    effects:
      clippy_config_file_created: true
    cost: 1
    status: complete
    completed_at: "2026-04-30"
    file: .clippy.toml
    description: |
      Create .clippy.toml with cognitive complexity threshold
      and function line limits.

  - name: add_cargo_lints_section
    preconditions:
      edition_2024: true
    effects:
      cargo_lints_section_added: true
    cost: 2
    status: complete
    completed_at: "2026-04-30"
    file: Cargo.toml
    description: |
      Add [lints.rust] and [lints.clippy] tables to Cargo.toml
      per Rust 2024 best practices. Configure explicit lint levels.

  - name: update_ci_clippy_flags
    preconditions:
      clippy_config_file_created: true
    effects:
      clippy_ci_flags_consistent: true
    cost: 1
    status: complete
    completed_at: "2026-04-30"
    file: .github/workflows/ci.yml
    description: |
      Update CI clippy command to include -W clippy::all
      for consistency with local development.
```

---

## Rationale

### Why `.clippy.toml`?

- Configures lint thresholds (cognitive complexity, line limits)
- Project-level defaults without per-file boilerplate
- Well-supported by clippy since 1.68+

### Why `Cargo.toml` lints?

- Rust 2024 feature - declarative lint configuration
- Replaces scattered `#![allow(...)]` attributes
- Enforced at build time, not just CI time

### Why Warn on `unwrap_used`/`expect_used`?

- Production code should use Result-based error handling
- Current code already follows this pattern (all unwraps in test code)
- Lint ensures new code follows same standards

---

## Success Criteria (All Complete ✓)

- [x] `.clippy.toml` created and valid
- [x] `Cargo.toml` has `[lints.rust]` and `[lints.clippy]` tables
- [x] CI clippy flags updated
- [x] All tests pass after changes
- [x] No new clippy warnings introduced