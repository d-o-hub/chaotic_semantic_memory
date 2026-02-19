# ADR-0038: Cargo.toml Modernization for crates.io Publishing

## Status

Implemented

## Context and Problem Statement

The `chaotic_semantic_memory` crate is approaching its 1.0 release and needs to be published to crates.io. Current Cargo.toml configuration has several gaps that prevent publishing and doesn't follow 2026 best practices:

**Publishing Blockers:**
- Missing required fields: `description`, `license`, `repository`
- Missing recommended fields: `keywords`, `categories`
- No resolver specification (defaults to older resolver)
- Missing `include`/`exclude` for package size optimization

**Best Practice Gaps:**
- Edition 2021 (2024 is latest stable)
- MSRV 1.82 (needs 1.85 for edition 2024)
- Dependency versions use minor-only ("1.0" instead of "1.0.219")
- CLI dependencies unconditionally included for library users
- Unused `exitcode` crate (ADR-0036 identifies this)

**Constraints from AGENTS.md:**
- Must not have hardcoded settings
- All fallible APIs return `Result<T, Error>`
- WASM threading paths must be gated
- No magic numbers without named constants

## Decision Drivers

1. **crates.io Compliance**: Must meet registry requirements for publishing
2. **2026 Best Practices**: Follow current Rust packaging standards
3. **Backward Compatibility**: Don't break existing users when possible
4. **Dependency Hygiene**: Remove unused deps, gate optional ones
5. **Reproducibility**: Pin dependency versions for consistent builds
6. **Wave 10 Alignment**: Coordinate with ADR-0036 (CI/DX Hardening) work

## Considered Options

### Option 1: Minimal Changes (Status Quo + Metadata Only)

Add only required crates.io metadata without edition upgrade.

**Pros:**
- Lowest risk
- No MSRV bump
- Fastest to implement

**Cons:**
- Doesn't address technical debt (exitcode, CLI deps)
- Stays on older edition
- Requires another change later for edition upgrade
- Doesn't align with Wave 10 goals

### Option 2: Comprehensive Modernization (Chosen)

Batch all improvements: edition 2024, metadata, dependency updates, CLI gating, exitcode removal.

**Pros:**
- One-time effort
- Meets all 2026 best practices
- Enables immediate crates.io publishing
- Aligns with ADR-0036 Phase 22 work
- Future-proof

**Cons:**
- Higher risk (more changes)
- MSRV bump excludes some users
- Requires thorough testing

### Option 3: Staged Approach

Do metadata first, edition upgrade later, CLI changes in separate PR.

**Pros:**
- Easier to bisect issues
- Lower risk per change
- Can ship metadata quickly

**Cons:**
- Three PRs instead of one
- Testing overhead multiplied
- Cargo.toml churn over time
- Doesn't align with swarm efficiency goals

## Decision Outcome

Chosen option: **Option 2 - Comprehensive Modernization**

We will implement all changes in a single coordinated effort, committed in logical steps for traceability. This aligns with Wave 10's focus on production hardening and CI/DX improvements (ADR-0036).

### Changes Overview

#### 1. Package Metadata (Required for crates.io)

```toml
[package]
description = "AI memory systems with hyperdimensional vectors and chaotic reservoirs"
license = "MIT"
repository = "https://github.com/d-o-hub/chaotic_semantic_memory"
homepage = "https://github.com/d-o-hub/chaotic_semantic_memory"
documentation = "https://docs.rs/chaotic_semantic_memory"
readme = "README.md"
keywords = ["ai", "memory", "hypervector", "reservoir", "wasm"]
categories = ["data-structures", "algorithms", "wasm"]
```

#### 2. Edition and MSRV Update

```toml
edition = "2024"
rust-version = "1.85"  # Required for edition 2024
resolver = "3"  # Latest resolver
```

**Rationale for Edition 2024:**
- No macro_rules! in codebase = no pat/pat_param migration issues
- SIMD unsafe blocks unchanged by edition
- Rayon par_iter not affected by closure capture changes
- Provides better borrow checker precision
- Rust 1.93.0 (current toolchain) fully supports it

**MSRV Justification:**
- 1.85 released Jan 2025 (8+ months ago)
- Edition 2024 requires 1.85 minimum
- Pre-1.0 crate can bump MSRV more freely
- Most production users follow stable or recent versions

#### 3. Dependency Version Pinning

Update from minor-only to specific patch versions for reproducibility:

```toml
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.138"
bincode = "1.3.3"
thiserror = "2.0.11"
tracing = "0.1.41"
tracing-subscriber = "0.3.19"
rand = "0.8.5"
clap = { version = "4.5.27", features = ["derive", "env", "string"] }
clap_complete = "4.5.42"
anyhow = "1.0.95"
colored = "2.2.0"
# exitcode REMOVED (ADR-0036)
```

#### 4. CLI Dependencies Gating (ADR-0036)

Move CLI-specific deps to target-specific section with feature flag:

```toml
[features]
default = ["cli"]
cli = ["dep:clap", "dep:clap_complete", "dep:anyhow", "dep:colored"]
wasm = []

[dependencies]
# CLI deps now optional
clap = { version = "4.5.27", features = ["derive", "env", "string"], optional = true }
clap_complete = { version = "4.5.42", optional = true }
anyhow = { version = "1.0.95", optional = true }
colored = { version = "2.2.0", optional = true }

# exitcode REMOVED - unused, CLI defines own ExitCode
```

**Backward Compatibility Strategy:**
- `cli` feature is **enabled by default**
- Existing users unaffected (default features include CLI)
- Library-only users can opt-out: `chaotic_semantic_memory = { version = "0.1", default-features = false }`

#### 5. Package Size Optimization

Add `include` to limit published package size:

```toml
include = [
    "/src",
    "/ benches",
    "/examples",
    "/tests",
    "README.md",
    "LICENSE",
    "CHANGELOG.md",
    "Cargo.toml",
]
```

This excludes:
- `.github/` (CI configs not needed in package)
- `plans/` (internal documentation)
- `scripts/` (development scripts)
- `.cargo/` (local aliases)
- `fuzz/` (fuzzing targets)
- `progress/` (internal tracking)
- `docs/architecture/` (architecture docs)

### Positive Consequences

1. **Publishable**: Meets all crates.io requirements
2. **Modern**: Edition 2024, resolver 3, latest best practices
3. **Reproducible**: Pinned dependency versions
4. **Flexible**: Library users can exclude CLI deps
5. **Clean**: Removes unused exitcode crate
6. **Sized**: Smaller published package via `include`
7. **Aligned**: Satisfies ADR-0036 Phase 22 requirements

### Negative Consequences

1. **MSRV Bump**: Users on Rust <1.85 cannot compile
2. **Feature Flag Complexity**: Adds feature resolution overhead
3. **Testing Surface**: Must test with/without `cli` feature
4. **Documentation**: Must document feature flags for users
5. **Churn**: Single large change vs incremental updates

## Pros and Cons of Options

### Option 1: Minimal Changes
- Good, because low risk and fast
- Bad, because doesn't address technical debt
- Bad, because defers necessary work

### Option 2: Comprehensive (Chosen)
- Good, because production-ready in one go
- Good, because aligns with Wave 10 goals
- Good, because future-proofs the crate
- Bad, because higher risk (mitigated with testing)
- Bad, because MSRV bump affects some users

### Option 3: Staged
- Good, because easier debugging
- Bad, because multiplied overhead
- Bad, because conflicts with swarm efficiency

## Implementation Plan

### Phase 1: Validation (30 minutes)

1. Verify edition 2024 compatibility:
   ```bash
   cargo check --edition 2024 --all-targets
   cargo check --target wasm32-unknown-unknown --edition 2024
   ```

2. Establish test baseline:
   ```bash
   cargo test --all-targets
   cargo clippy --all-targets --all-features -- -D warnings
   ```

### Phase 2: Implementation (2 hours)

Commits in order:

1. **commit**: Add crates.io metadata (description, license, repository, etc.)
2. **commit**: Add resolver = "3" and include field
3. **commit**: Update dependency versions to specific patches
4. **commit**: Remove exitcode crate
5. **commit**: Gate CLI deps with `cli` feature
6. **commit**: Update MSRV to 1.85 and edition to 2024

### Phase 3: Testing (1 hour)

Test matrix:
- [ ] `cargo check --all-targets --all-features`
- [ ] `cargo check --target wasm32-unknown-unknown`
- [ ] `cargo test --all-targets`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo run --bin csm -- --help` (CLI works)
- [ ] `cargo check --no-default-features` (library-only builds)
- [ ] `cargo publish --dry-run --allow-dirty`

### Phase 4: Documentation (30 minutes)

- Create this ADR (0038)
- Update GOAP_STATE.md:
  - `cargo_toml_modernized: true`
  - `crates_io_ready: true`
  - `edition_2024: true`
- Update ACTIONS.md: Mark Phase 22 items complete:
  - `remove_exitcode_crate: complete`
  - `gate_cli_deps: complete`
- Update ADR_REGISTRY.md

## Related ADRs

- **ADR-0036**: CI/DX Hardening (requires exitcode removal and CLI deps gating)
- **ADR-0037**: Rust Best Practices (aligns with edition upgrade)
- **ADR-0010**: Public API Result Contract (maintained with these changes)

## References

- [Rust Edition 2024 Guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
- [Cargo Manifest Format](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [crates.io Publishing Requirements](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [AGENTS.md - Hard Constraints](/home/do/git/chaotic_semantic_memory/AGENTS.md)

## Notes

**Analysis performed by:** Analysis Swarm (RYAN, FLASH, SOCRATES)  
**Consensus:** Unanimous approval with documented mitigations  
**Risk Level:** Low (no macro_rules!, SIMD unchanged, comprehensive testing plan)

---

**Created:** 2026-02-19  
**Author:** Analysis Swarm  
**Status:** Accepted (pending implementation)
