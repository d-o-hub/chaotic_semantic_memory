# ADR-0090: Harness Engineering & rust-2026-template Alignment

## Status

Accepted (2026-06-23)

## Context

The [rust-2026-template](https://github.com/d-oit/rust-2026-template) (v0.3.2, 392 commits)
represents the current best-practice baseline for Rust projects in the d-o-hub organization.
A gap analysis against `chaotic_semantic_memory` reveals 15 missing infrastructure components
that the template considers standard. This ADR documents what to adopt, what to defer, and
the rationale for each decision.

### Template Features (Key)

| Feature | Template Path | Purpose |
|---------|--------------|---------|
| HARNESS.md | root | Feedforward/feedback engineering loop documentation |
| deny.toml | root | Supply chain auditing (licenses + advisories) |
| .pre-commit-config.yaml | root | Standardized git hooks via pre-commit framework |
| rust-toolchain.toml | root | Pinned toolchain (MSRV 1.88) |
| scripts/quality-gates.sh | scripts/ | Unified local quality gate runner |
| scripts/harness-check.sh | scripts/ | Structured error output for agents |
| .codecov.yml | root | Code coverage configuration |
| .gitleaks.toml | root | Secret scanning configuration |
| commitlint.config.cjs | root | Conventional commit enforcement |
| dist-workspace.toml | root | cargo-dist distribution configuration |
| tests/arch_fitness.rs | tests/ | Architecture fitness tests (LOC gate, layer violations) |
| cargo-nextest | CI | Faster test runner with per-test isolation |
| insta snapshots | tests/ | Snapshot/golden-file testing |
| .agents/context/ | .agents/ | Cross-repo conventions for derived repos |
| Makefile | root | Developer ergonomics (make test, make lint, etc.) |

### Current State in chaotic_semantic_memory

| Feature | Status | Notes |
|---------|--------|-------|
| HARNESS.md | ❌ Missing | No harness engineering framework |
| deny.toml | ❌ Missing | No supply chain auditing |
| .pre-commit-config.yaml | ❌ Missing | Have scripts/pre-commit.sh (custom) |
| rust-toolchain.toml | ❌ Missing | MSRV 1.85 in Cargo.toml only |
| scripts/quality-gates.sh | ❌ Missing | Have scripts/validate.sh (equivalent) |
| scripts/harness-check.sh | ❌ Missing | No structured agent error output |
| .codecov.yml | ❌ Missing | No coverage tracking service |
| .gitleaks.toml | ❌ Missing | No secret scanning |
| commitlint.config.cjs | ❌ Missing | Conventional commits by convention only |
| dist-workspace.toml | ❌ Missing | Using custom release workflow |
| tests/arch_fitness.rs | ❌ Missing | LOC gate in scripts only |
| cargo-nextest | ❌ Missing | Using cargo test |
| insta snapshots | ❌ Missing | No snapshot tests |
| .agents/context/ | ❌ Missing | Skills exist but no cross-repo context |
| Makefile | ❌ Missing | Using scripts/ directory |
| .clippy.toml | ✅ Exists | Configured with pedantic lints |
| llms.txt | ✅ Exists | Token-efficient LLM context |
| .shellcheckrc | ✅ Exists | Shell script linting |
| scripts/pre-commit.sh | ✅ Exists | Custom pre-commit (fmt + LOC) |
| scripts/validate.sh | ✅ Exists | Full validation gate |

## Decision

### Phase 1: High-Impact, Low-Cost (Wave 29 — cost 18)

Adopt immediately — these close security/quality gaps with minimal risk:

1. **HARNESS.md** (cost 3): Create harness map adapted for HDC/reservoir domain.
   Maps existing sensors (clippy, tests, validate.sh) and guides (AGENTS.md, skills).
   Adds feedforward/feedback loop documentation for agent self-correction.

2. **deny.toml** (cost 3): Supply chain auditing. Configure cargo-deny for:
   - License allowlist (MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib)
   - Advisory database checks (rustsec)
   - Ban duplicate crate versions where feasible
   - Document known unmaintained crates (bincode 1.x, per UNMAINTAINED_CRATES.md)

3. **rust-toolchain.toml** (cost 1): Pin toolchain to 1.88.0 stable.
   Bump MSRV from 1.85 → 1.88 in Cargo.toml. Enables Rust 2024 edition
   features fully (gen blocks, let chains, etc.).

4. **scripts/quality-gates.sh** (cost 2): Unified gate script that wraps validate.sh
   with structured output. Adds cargo-deny check to the pipeline.

5. **scripts/harness-check.sh** (cost 2): Agent-optimized error output with
   `HARNESS VIOLATION` prefix and fix hints. Wraps quality-gates.sh sensors.

6. **.gitleaks.toml** (cost 1): Secret scanning config. Add to pre-commit pipeline.
   Critical for a crate that handles database credentials (Turso tokens).

7. **tests/arch_fitness.rs** (cost 3): Compile-time architecture fitness tests:
   - LOC gate (all src/ files ≤ 500 LOC)
   - Module dependency layering (persistence → singularity → framework)
   - No `unsafe` outside hyperdim_simd.rs
   - Public API surface stability check

8. **.agents/context/shared-conventions.md** (cost 3): Cross-repo context document
   for d-o-hub organization conventions. Commit format, branch naming, PR requirements,
   quality thresholds.

### Phase 2: Medium-Impact (Wave 30 — cost 14, delegated to Jules)

Adopt after Phase 1 lands:

9. **.pre-commit-config.yaml** (cost 4): Migrate from custom scripts/pre-commit.sh
   to standardized pre-commit framework. Add hooks: fmt, clippy, deny, gitleaks,
   shellcheck, commitlint. Preserves existing hook behavior.

10. **cargo-nextest** (cost 3): Add nextest as test runner in CI. Faster parallel
    execution, per-test timeout, JUnit XML output for CI visibility.
    Keep `cargo test` as fallback for environments without nextest.

11. **.codecov.yml** (cost 3): Code coverage tracking. Configure with:
    - Target: 70% line coverage (current estimate)
    - Patch coverage: 80% for new code
    - Add llvm-cov to CI pipeline

12. **commitlint.config.cjs** (cost 2): Enforce conventional commits. Scope list:
    singularity, reservoir, framework, persistence, cli, wasm, retrieval, embedding,
    mcp, observability, bridge, duckdb.
    Pair with `.pre-commit-config.yaml` husky/commitlint hook.

13. **dist-workspace.toml** (cost 2): cargo-dist configuration for automated
    binary distribution. Alternative to current release-manager.sh for pre-built
    binaries across platforms.

### Deferred (Not Adopting Now)

14. **insta snapshot tests** (cost 6): Defer until a clear regression pattern
    emerges that golden-file tests would catch better than existing property-based
    and integration tests. Current 773 tests provide adequate regression coverage.

15. **Makefile** (cost 1): Defer — scripts/ directory pattern is established and
    documented. Adding Makefile creates parallel discovery paths.

## Consequences

### Positive

- Supply chain security via cargo-deny (rustsec advisories, license compliance)
- Harness engineering documentation enables agent self-correction loops
- Pinned toolchain eliminates "works on my machine" MSRV drift
- Architecture fitness tests catch structural regressions at compile time
- Structured error output (harness-check.sh) improves agent iteration speed
- Secret scanning prevents credential leaks (critical for Turso tokens)

### Negative

- MSRV bump to 1.88 may break consumers pinned to older Rust (mitigated: edition 2024 already requires 1.85+)
- cargo-deny adds ~5s to CI pipeline (acceptable)
- pre-commit framework is Node.js-based (adds npm devDependency for hooks)

### Risks

- deny.toml may flag transitive dependencies we cannot control (mitigation: documented exceptions)
- Architecture fitness tests may become brittle if module structure changes frequently (mitigation: test stability via ADR review)

## Implementation Plan

```
Wave 29 (Phase 1): 8 items, cost 18
  ├── HARNESS.md (cost 3)
  ├── deny.toml + CI integration (cost 3)
  ├── rust-toolchain.toml + MSRV bump (cost 1)
  ├── scripts/quality-gates.sh (cost 2)
  ├── scripts/harness-check.sh (cost 2)
  ├── .gitleaks.toml (cost 1)
  ├── tests/arch_fitness.rs (cost 3)
  └── .agents/context/shared-conventions.md (cost 3)

Wave 30 (Phase 2): 5 items, cost 14 — delegated to Jules
  ├── .pre-commit-config.yaml (cost 4)
  ├── cargo-nextest CI (cost 3)
  ├── .codecov.yml + llvm-cov (cost 3)
  ├── commitlint.config.cjs (cost 2)
  └── dist-workspace.toml (cost 2)
```

## Implementation matrix (reconciled 2026-07-16)

| Template item | Repo artifact | Status |
|---|---|---|
| HARNESS.md | `HARNESS.md` (162 lines) | **done** |
| deny.toml + CI | `deny.toml` + `cargo-deny` job in `ci.yml` | **done** |
| rust-toolchain / MSRV | workspace `rust-version = 1.88` | **done** |
| quality gates script | `scripts/validate.sh`, `scripts/harness-check.sh` | **done** |
| pre-commit config | `.pre-commit-config.yaml` (fmt commit / clippy+deny push) | **done** |
| commitlint | `commitlint.config.cjs` | **done** |
| gitleaks | pre-commit gitleaks hook | **done** |
| arch fitness tests | `tests/arch_fitness.rs` (if present) / LOC gate | **partial** |
| nextest CI | not required; cargo test matrix | **deferred** |
| codecov | optional; not blocking Wave 32 | **deferred** |
| skill catalog + evals | `scripts/generate-skill-catalog.sh`, `scripts/eval-critical-skills.sh` | **done** (2026-07-16) |
| plan archive manifest | `scripts/plans-archive-manifest.sh` | **done** (2026-07-16) |

This matrix supersedes the 2026-06-23 baseline checklist for planning purposes.
`harness_engineering_state_truthful` may be set true when CI green includes the
new skill catalog/eval gates on main.

## References

- [rust-2026-template](https://github.com/d-oit/rust-2026-template) — Source template
- [HARNESS.md in template](https://github.com/d-oit/rust-2026-template/blob/main/HARNESS.md) — Harness engineering spec
- [Martin Fowler: Harness Engineering](https://martinfowler.com/articles/harness-engineering.html) — Concept origin
- ADR-0077: Clippy pedantic selective promotion (related quality tooling)
- ADR-0036: CI/DX hardening (pre-commit hooks, LOC gate)
- plans/UNMAINTAINED_CRATES.md — Known unmaintained deps (deny.toml exceptions)
