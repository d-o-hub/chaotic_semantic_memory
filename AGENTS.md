# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

## Hard Constraints
- Source files: `<= 500 LOC` each.
- `SKILL.md` (.agents/skills/ folder): `<= 250 LOC`; move detail to `reference/`, `scripts/`, or `assets/`.
- Use `libsql` (never `turso-client`).
- Use Tokio async/await for I/O.
- Use Rayon for CPU parallelism.
- All public APIs return `Result<T, Error>`.
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- WASM threading paths must be gated with `#[cfg(not(target_arch = "wasm32"))]`.

## Key Files
- @Cargo.toml — dependencies and features
- @src/lib.rs — crate root and prelude
- @plans/GOAP_STATE.md — current world state
- @plans/ACTIONS.md — GOAP action plan
- @.github/workflows/ci.yml — CI pipeline

## Skills
- `rust-development`: Implement or refactor Rust modules
- `testing-validation`: Run compile/test/lint/LOC gates
- `benchmarking-perf`: Criterion benchmarks and performance targets
- `debugging-reservoir`: Diagnose ESN spectral radius, sparse weights, dynamics
- `adr-creation`: Write architecture decision records
- `goap-planning`: Build ordered action plans from state to goal
- `github-ci-guardrails`: Validate merge readiness via `gh` CLI

## Accuracy Guardrails
- Do not assume crate existence/version; verify.
- When uncertain on modern Rust practice, verify with web research.
- If a decision changes architecture, write/update ADR in `plans/adr/`.
- Prefer exact, testable instructions over high-level advice.

## Git + CI Source of Truth
- Use atomic commits: one logical change per commit.
- Before commit, run: `cargo check`, `cargo test --all-features`, `cargo fmt --check`, `cargo clippy -- -D warnings`.
- Treat GitHub Actions as merge gate source of truth.
- Use `gh` CLI to verify checks: `gh pr checks --watch`, `gh run list --branch <branch> --limit 5`.
- Do not claim success until local checks and relevant GitHub checks pass.

## Performance Gate
- Save baseline: `cargo bench --bench benchmark -- --save-baseline main`
- Compare: `cargo bench --bench benchmark -- --baseline main`
- Target: `reservoir_step_50k < 100μs`.

## Learning Loop (RALPH)
After each iteration:
1. Record what worked in @progress/LEARNINGS.md.
2. Record progress in @progress/PROGRESS.md.
3. Update module LOC counts.
4. Run test + bench gates.
5. Commit message format: `RALPH iteration N: [summary]`.
