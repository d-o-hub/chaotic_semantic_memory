# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

## Hard Constraints
- Source files: `<= 500 LOC` each.
- `SKILL.md` (`.agents/skills/` folder): `<= 250 LOC`; detailed references in `reference/`, `scripts/`, or `assets/`.
- Use `libsql` (never `turso-client`).
- Use Tokio async/await for I/O.
- Use Rayon for CPU parallelism.
- All fallible public APIs return `Result<T, Error>`.
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- WASM threading paths must be gated with `#[cfg(not(target_arch = "wasm32"))]`.
- No hardcoded runtime settings or magic numbers in production paths; use named constants and configurable env/config values.

## Key Files and Folders
- @Cargo.toml — dependencies and features
- @src/lib.rs — crate root and prelude
- @plans/GOAP_STATE.md — current world state
- @plans/GOALS.md — project goals and targets
- @plans/ACTIONS.md — GOAP action plan
- @.github/workflows/ci.yml — CI pipeline
- @plans/adr/ — ADR folder
- @docs/architecture/context.yaml — Structured LLM context (machine-optimized)
- @progress/LEARNINGS.md — Self-learning patterns and iteration history
- @progress/PROGRESS.md — Project progress tracking

## Skills (13 Total)

### Core Skills
- `rust-development`: Implement or refactor Rust modules
  - **References**: @.agents/skills/rust-development/reference/codebase-patterns.md
  - **Scripts**: @.agents/skills/rust-development/scripts/validate.sh
- `testing-validation`: Run compile/test/lint/LOC gates
  - **Scripts**: @.agents/skills/testing-validation/scripts/validate.sh, @.agents/skills/testing-validation/scripts/loc-check.sh
- `benchmarking-perf`: Criterion benchmarks and performance targets
- `debugging-reservoir`: Diagnose ESN spectral radius, sparse weights, dynamics
- `skill-memory`: Use csm CLI for skill learning and knowledge graphs
  - **References**: @.agents/skills/skill-memory/references/integration-patterns.md, @.agents/skills/skill-memory/references/api-reference.md
- `adr-creation`: Write architecture decision records
  - **References**: @.agents/skills/adr-creation/references/madr-template.md, @.agents/skills/adr-creation/references/review-checklist.md
- `goap-planning`: Build ordered action plans from state to goal
  - **References**: @.agents/skills/goap-planning/references/planner-pattern.md, @.agents/skills/goap-planning/references/action-model.md
- `github-ci-guardrails`: Validate merge readiness via `gh` CLI
  - **References**: @.agents/skills/github-ci-guardrails/references/local-gates.md, @.agents/skills/github-ci-guardrails/references/gh-ci-truth.md
- `drawio`: Create architecture diagrams for plans, modules, and data flows
- `git-workflow`: Git commit conventions, validation gates, CI/CD workflows
  - **References**: @.agents/skills/git-workflow/references/commit-types.md
- `release-management`: GitHub release management, crates.io publishing
  - **References**: @.agents/skills/release-management/references/version-tag-format.md, @.agents/skills/release-management/references/trusted-publishing.md
  - **Scripts**: @.agents/skills/release-management/scripts/create-github-release.sh, @.agents/skills/release-management/scripts/validate-release.sh

### Swarm Group Skills (Parallel Execution)
- `swarm-testing-quality`: Property-based testing, fuzzing, edge case coverage
- `swarm-performance`: SIMD optimization, connection pooling, batch APIs, caching
- `swarm-observability`: Tracing, metrics, error context
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
- **Never create unused code**: Before adding proc-macros, traits, or convenience APIs, verify at least one real usage site exists in examples, tests, or docs.

## Quick Reference

### Validation Gates
Run before commit (see `git-workflow` skill for details):
```bash
scripts/validate.sh
```

### Auto-generate AI docs
```bash
scripts/gen-llms-txt.sh  # generates llms.txt and llms-full.txt
```
This runs automatically on post-commit when source files change.

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

## Self-Learning Patterns

Key patterns recorded from iterations (see @progress/LEARNINGS.md for full history):

### What Works
1. Systematic codebase analysis before planning — found more real issues than GOAP state listed
2. Using oracle for deep code review across all modules simultaneously
3. Writing ADRs for every non-trivial architectural change before implementation
4. Creating domain-specific debugging skills rather than generic boilerplate
5. Adding executable scripts to skills — agent can run them directly
6. Treating GOAP state booleans as executable acceptance criteria
7. Using seeded RNG (`StdRng::seed_from_u64(42)`) in tests for determinism
8. Migrating to `libsql::Builder` to remove deprecated API usage
9. Enabling `PRAGMA foreign_keys = ON` per-connection for deterministic FK behavior

### Technical Insights
- Dense `Array2<f32>` for 50k×50k reservoir is infeasible (~10 GB). CSR with k=64 reduces to ~25 MB.
- `HVec10240::permute()` with `bit_shift == 0` causes undefined behavior — must guard
- `Arc<RwLock<Connection>>` for libsql is unsafe under tokio. Per-operation `connect()` is cheap and eliminates Send/Sync risks
- Always use `f32::total_cmp()` for similarity sorting — `partial_cmp().unwrap()` panics on NaN
- `Vec<Vec<(usize, f32)>>` incurs substantial allocator overhead; contiguous CSR buffers are faster
- For large sparse reservoirs, memory locality can dominate runtime more than arithmetic throughput

### What to Avoid
- Do not use dense matrices for reservoirs > ~2000 nodes
- Do not share a single libsql `Connection` across async tasks via RwLock
- Do not use `partial_cmp().unwrap()` on floats
- Do not assume `Vec<(String, f32)>` associations deduplicate — use `HashMap<String, f32>`
- Do not use `cargo bench -- --baseline` (without `--bench benchmark`) — libtest benches interfere
- Do not suppress deprecated libsql constructors long-term — migrate to `Builder`
- Do not relax spectral-radius guardrails to chase speed
- Do not pool connections for local SQLite (no benefit, adds overhead)
- Do not make versioning mandatory (should be opt-in)

## Learning Loop
After each iteration:
1. Record what worked in @progress/LEARNINGS.md.
2. Record progress in @progress/PROGRESS.md.
3. Update module LOC counts.
4. Run test + bench gates.
5. Commit with Conventional Commits format (see `git-workflow` skill).

## Skill Memory (Dogfooding CSM)

Skills use the `csm` CLI to persist learning and build knowledge graphs.

### Configuration
```yaml
memory:
  enabled: true
  database: ".agents/memory/skill-memory.db"
  namespace_prefix: "skill"
```

### Quick Usage
```bash
source scripts/skill-memory/skill-memory.sh

# Remember operation
CONCEPT_ID=$(skill_remember "adr-creation" "decision" "ADR-0043" "approved")

# Recall similar
skill_recall "CSM integration" 0.7 5

# Create association
skill_associate "error::xyz" "solution::abc" 0.95
```

### Available Functions
- `skill_remember skill op context result` - Store operation
- `skill_recall query [threshold] [top_k]` - Find similar
- `skill_associate c1 c2 [strength]` - Link concepts
- `skill_related concept_id [min_strength]` - Get related
- `skill_suggest query [threshold]` - Show suggestions

### Dogfooding Principle
By using the `csm` CLI for skill memory, we validate:
- CLI reliability in real workflows
- libsql persistence durability
- Edge cases through actual usage
- Framework utility through self-use
