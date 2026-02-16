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

## Key Files
- @Cargo.toml — dependencies and features
- @src/lib.rs — crate root and prelude
- @plans/GOAP_STATE.md — current world state
- @plans/GOALS.md — project goals and targets
- @plans/ACTIONS.md — GOAP action plan
- @.github/workflows/ci.yml — CI pipeline

## Skills

### Core Skills
- `rust-development`: Implement or refactor Rust modules
- `testing-validation`: Run compile/test/lint/LOC gates
- `benchmarking-perf`: Criterion benchmarks and performance targets
- `debugging-reservoir`: Diagnose ESN spectral radius, sparse weights, dynamics
- `adr-creation`: Write architecture decision records
- `goap-planning`: Build ordered action plans from state to goal
- `github-ci-guardrails`: Validate merge readiness via `gh` CLI
- `drawio`: Create architecture diagrams for plans, modules, and data flows

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

## Git + CI Source of Truth
- Use atomic commits: one logical change per commit.
- Before commit, run: `CARGO_TERM_PROGRESS_WHEN=never cargo check --message-format=short`, `cargo test --all-features --quiet` (or `cargo nextest run --all-features`), `cargo fmt --check`, `cargo clippy -- -D warnings`.
- Treat GitHub Actions as merge gate source of truth.
- Use `gh` CLI to verify checks: `gh pr checks --watch`, `gh run list --branch <branch> --limit 5`.
- Do not claim success until local checks and relevant GitHub checks pass.

## Performance Gate
- Save baseline: `cargo bench --bench benchmark -- --save-baseline main`
- Compare: `cargo bench --bench benchmark -- --baseline main`
- Target: `reservoir_step_50k < 100μs`.

## Learning Loop
After each iteration:
1. Record what worked in @progress/LEARNINGS.md.
2. Record progress in @progress/PROGRESS.md.
3. Update module LOC counts.
4. Run test + bench gates.
5. Commit with Conventional Commits format: `<type>(<scope>): <description>`

### Commit Message Format (Conventional Commits)

Use [Conventional Commits](https://www.conventionalcommits.org/) for atomic, readable history:

```
<type>(<scope>): <short summary in imperative mood>

<body: explain what and why, not how>

<footer: BREAKING CHANGE, Co-authored-by, etc.>
```

**Types:**
- `feat`: New feature or capability
- `fix`: Bug fix or correction
- `perf`: Performance improvement
- `refactor`: Code change with no behavior change
- `test`: Adding or fixing tests
- `docs`: Documentation changes (AGENTS.md, ADRs, README)
- `chore`: Maintenance (deps, CI, formatting)

**Scopes:**
- `hyperdim`: Hypervector operations (`src/hyperdim.rs`)
- `reservoir`: Echo state network (`src/reservoir.rs`)
- `singularity`: Concept store (`src/singularity.rs`)
- `persistence`: libSQL storage (`src/persistence.rs`)
- `framework`: High-level API (`src/framework.rs`)
- `wasm`: WASM bindings (`src/wasm.rs`)
- `ci`: CI/CD pipeline
- `skills`: Agent skills (`.agents/skills/`)
- `adr`: Architecture decisions (`plans/adr/`)
- `planning`: GOAP state, actions, goals (`plans/`)

**Examples:**
```bash
# Feature with scope
feat(reservoir): add SIMD-accelerated cosine similarity

# Fix with breaking change footer
fix(persistence): enforce foreign key constraints

BREAKING CHANGE: existing databases without FK support may fail

# Multiple scopes in body
perf(hyperdim,framework): optimize batch operations

- Use par_chunks for hypervector bundling
- Add inject_concepts() batch API

# Documentation update
docs(adr): add ADR-0013 for SIMD hypervector operations

# Chore/maintenance
chore(deps): update libsql to 0.5
```
