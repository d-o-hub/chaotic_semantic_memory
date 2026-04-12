# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

## Hard Constraints
See: [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md)

## Key Files and Folders
- @Cargo.toml — dependencies and features
- @src/lib.rs — crate root and prelude
- @src/singularity.rs — Core concept storage and similarity search
- @src/reservoir.rs — Echo State Network implementation
- @src/semantic_bridge.rs — Semantic Bridge Layer (ADR-0061)
- @src/bridge_retrieval.rs — Bridge retrieval pipeline
- @src/retrieval/ — Hybrid BM25/HDC retrieval (ADR-0062)
- @plans/GOAP_STATE.md — current world state
- @plans/GOALS.md — project goals and targets
- @plans/ACTIONS.md — GOAP action plan
- @.github/workflows/ci.yml — CI pipeline
- @plans/adr/ — ADR folder
- @docs/architecture/context.yaml — Structured LLM context
- @progress/LEARNINGS.md — Self-learning patterns
- @progress/PROGRESS.md — Project progress tracking

## Skills (19 Total)

### Core Skills
- `rust-development`: Implement or refactor Rust modules
- `testing-validation`: Run compile/test/lint/LOC gates
- `goap-planning`: Build ordered action plans from state to goal
- `adr-creation`: Write architecture decision records
- `github-ci-guardrails`: Validate merge readiness via gh CLI
- `git-workflow`: Git commit conventions, validation gates, CI/CD
- `release-management`: GitHub release management, crates.io publishing
- `benchmarking-perf`: Criterion benchmarks and performance targets
- `debugging-reservoir`: Diagnose ESN spectral radius, sparse weights, dynamics
- `skill-memory-internal`: Internal dogfooding memory workflow via csm CLI
- `memory-lifecycle-verification`: Portable save/load/archive/delete verification for files and DB records
- `turso-memory-verification`: Verify memory persistence before releases (REQUIRED)
- `drawio`: Create architecture diagrams

### Swarm Group Skills (Parallel Execution)
- `swarm-testing-quality`: Property-based testing, fuzzing, edge case coverage
- `swarm-performance`: SIMD optimization, connection pooling, batch APIs, caching
- `swarm-observability`: Tracing, metrics, error context
- `swarm-advanced-features`: Export/import, versioning, migrations, backup/restore
- `analysis-swarm`: Multi-persona code analysis orchestrator

### Using Swarm Mode
1. Check @plans/SWARM_COORDINATION.md for current swarm status
2. Each swarm group operates independently on its phase
3. Group agents report progress to shared GOAP_STATE
4. Final integration happens at phase boundaries

## Accuracy Guardrails
See: [agents-docs/accuracy-guardrails.md](agents-docs/accuracy-guardrails.md)

## Quick Reference
See: [agents-docs/quick-reference.md](agents-docs/quick-reference.md)

## Self-Learning Patterns
See: [agents-docs/self-learning-patterns.md](agents-docs/self-learning-patterns.md)

## Skill Memory (Dogfooding CSM)
See: [agents-docs/skill-memory.md](agents-docs/skill-memory.md)
