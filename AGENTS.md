# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

## Hard Constraints
- Source files: `<= 500 LOC` each.
- `SKILL.md` (.agents/skills/ folder): `<= 250 LOC`; move detail to `reference/`, `scripts/`, or `assets/`.
- Use `libsql` (never `turso-client`).
- Use Tokio async/await for I/O.
- Use Rayon for CPU parallelism.
- All fallible public APIs return `Result<T, Error>`.
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- WASM threading paths must be gated with `#[cfg(not(target_arch = "wasm32"))]`.

## Key Files and folder
- @Cargo.toml — dependencies and features
- @src/lib.rs — crate root and prelude
- @plans/GOAP_STATE.md — current world state
- @plans/GOALS.md — project goals and targets
- @plans/ACTIONS.md — GOAP action plan
- @.github/workflows/ci.yml — CI pipeline
- @plans/adr/ — ADR folder

## Skills (13 Total)

### Core Skills
- `rust-development`: Implement or refactor Rust modules
- `testing-validation`: Run compile/test/lint/LOC gates
- `benchmarking-perf`: Criterion benchmarks and performance targets
- `debugging-reservoir`: Diagnose ESN spectral radius, sparse weights, dynamics
- `adr-creation`: Write architecture decision records
- `goap-planning`: Build ordered action plans from state to goal
- `github-ci-guardrails`: Validate merge readiness via `gh` CLI
- `drawio`: Create architecture diagrams for plans, modules, and data flows
- `git-workflow`: Git commit conventions, validation gates, CI/CD workflows

### Swarm Group Skills (Parallel Execution)
- `swarm-testing-quality`: Property-based testing, fuzzing, edge case coverage
- `swarm-performance`: SIMD optimization, connection pooling, batch APIs, caching
- `swarm-observability`: Tracing, metrics, derive macros, error context
- `swarm-advanced-features`: Export/import, versioning, migrations, backup/restore

### Using Swarm Mode
When executing in swarm mode:
1. Check @plans/SWARM_COORDINATION.md for current swarm status
2. Each swarm group operates independently on its phase
3. Group agents report progress to shared GOAP_STATE
4. Final integration happens at phase boundaries

## Accuracy Guardrails
- Do not assume crate existence/version; verify.
- When uncertain on modern Rust practice, verify with web research.
- If a decision changes architecture, write/update ADR in `plans/adr/`.
- Prefer exact, testable instructions over high-level advice.

## Quick Reference

### Validation Gates
Run before commit (see `git-workflow` skill for details):
```bash
scripts/validate.sh
```

### Performance Gate
```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```
Target: `reservoir_step_50k < 100μs`

### Commit Format
Use Conventional Commits (see `git-workflow` skill):
```
<type>(<scope>): <description>

<body>
```

## Learning Loop
After each iteration:
1. Record what worked in @progress/LEARNINGS.md.
2. Record progress in @progress/PROGRESS.md.
3. Update module LOC counts.
4. Run test + bench gates.
5. Commit with Conventional Commits format (see `git-workflow` skill).
