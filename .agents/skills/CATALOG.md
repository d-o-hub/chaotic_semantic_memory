# Skill Catalog

32 skills. Load with `skill` tool when task matches trigger.

## Core (15)

| Skill | Trigger |
|-------|---------|
| `rust-development` | Writing/modifying Rust source |
| `testing-validation` | Running validate.sh, cargo test/clippy/fmt |
| `github-ci-guardrails` | Pre-merge CI verification, `gh pr checks` |
| `git-workflow` | Committing, branching, PR creation |
| `goap-orchestrator` | Multi-issue execution, wave dispatch, PR triage |
| `goap-planning` | Building action plans with preconditions/effects |
| `release-management` | Tags, crates.io, npm publish, GitHub Releases |
| `dist-channel-selection` | Choosing cargo vs WASM npm vs CLI npm |
| `npm-trusted-publishers` | npm OIDC E404/provenance failures |
| `benchmarking-perf` | Criterion benches, hot-path optimization |
| `debugging-reservoir` | Spectral radius, chaotic dynamics, ESN tuning |
| `adr-creation` | Architecture decisions needing durable rationale |
| `skill-memory-internal` | Dogfooding csm CLI for engineering context |
| `memory-lifecycle-verification` | Save/load/archive/delete verification |
| `turso-memory-verification` | Turso/libSQL roundtrip before releases |

## Swarm (5)

| Skill | Trigger |
|-------|---------|
| `analysis-swarm` | RYAN/FLASH/SOCRATES multi-persona decisions |
| `swarm-testing-quality` | proptest, fuzzing, edge cases |
| `swarm-performance` | SIMD, pooling, batch APIs, caching |
| `swarm-observability` | Tracing, metrics, error context |
| `swarm-advanced-features` | Export/import, versioning, migrations |

## Workflow (4)

| Skill | Trigger |
|-------|---------|
| `learn` | End-of-session insight capture |
| `task-decomposition` | Breaking complex tasks into atomic goals |
| `shell-script-quality` | ShellCheck + BATS for scripts/ |
| `jules-orchestration` | Delegating to Jules CLI remote agent |

## Automation (5)

| Skill | Trigger |
|-------|---------|
| `self-fix-loop` | CI failure → classify → fix → retry |
| `iterative-refinement` | Red-green-refactor test loops |
| `skill-creator` | Creating new .agents/skills/ entries |
| `skill-evaluator` | Measuring skill effectiveness |
| `codacy` | Codacy static analysis gate failures |

## TRIZ (2)

| Skill | Trigger |
|-------|---------|
| `triz-analysis` | Design contradictions before implementation |
| `triz-solver` | Applying inventive principles after analysis |

## Visualization (1)

| Skill | Trigger |
|-------|---------|
| `drawio` | Architecture diagrams, data flow visualization |

---

## Consolidation Candidates

These pairs overlap and could merge if skill count becomes unwieldy:
- `memory-lifecycle-verification` + `turso-memory-verification` → `persistence-verification`
- `self-fix-loop` + `iterative-refinement` → `fix-loop`
- `goap-orchestrator` absorbs `task-decomposition` (already does wave decomposition)
