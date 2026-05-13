# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

## Session Workflow — PRE-FLIGHT → EXECUTE → VERIFY

### PRE-FLIGHT — Understand before acting
1. Load state: `@plans/GOAP_STATE.md` (action_last_completed, module LOC)
2. Review uncommitted changes: `git status --short && git diff HEAD`
3. LOC gate pre-check: `find src -name '*.rs' -exec wc -l {} + | sort -rn | head -20` (all ≤ 500)
4. CI baseline: `gh run list --workflow=ci.yml --limit 3`

### EXECUTE — Read, plan, implement
1. Read target files + neighbors before editing
2. For 3+ step tasks: plan in `plans/`, document approach, get approval
3. Match existing style, naming, conventions — see `@.agents/skills/rust-development/reference/codebase-patterns.md`
4. After changes: `./scripts/validate.sh` (first run: `--save-baseline` for baseline-aware mode)

### VERIFY — Gates before completion
1. `./scripts/validate.sh` — all quality gates in one command (baseline-aware)
2. Fix ALL new issues (pre-existing errors filtered by baseline; new regressions block merge)
3. Atomic commit: `./scripts/ai-commit.sh`
4. Branch → PR → CI: `git push origin <branch>`, `gh pr create`, `gh pr checks --watch`
5. Update `@plans/GOAP_STATE.md` with `action_last_completed` and new counts

## 8 Core Rules
1. **Always read before editing** — never guess file contents
2. **Stay under context limits** — each instruction must earn its place
3. **Validation gates mandatory** — `./scripts/validate.sh` before claiming completion
4. **Reference, don't duplicate** — point to source files via `@path/to/file`
5. **Plan before implementing** — for tasks with 3+ steps
6. **Encode errors immediately** — every correction becomes a rule in `agents-docs/hard-constraints.md`
7. **Never push directly to `main`** — branch → commit → PR → squash merge after CI
8. **CI passes for your changes** — baseline filters pre-existing errors; new regressions block merge before PR

## Coding Standards (DeepSource Parity)
See `@.agents/skills/rust-development/reference/codebase-patterns.md` for full conventions.
- `Default::default()` constructs struct directly; `new()` delegates to `default()` (not vice-versa)
- Use `.map_or()` / `.is_some_and()` instead of `.map().unwrap_or()`
- `clippy::map_unwrap_or` promoted to `warn` in Cargo.toml

## Hard Constraints
See `@agents-docs/hard-constraints.md` — LOC ≤ 500, spectral radius [0.9, 1.1], libsql only, Tokio async I/O, Rayon gated `#[cfg(not(target_arch = "wasm32"))]`.

## Release Safety
See `@.agents/skills/release-management/SKILL.md`. Critical: never release with failing CI. `./scripts/validate.sh` passes on all platforms before tag.

## Key Files
**Core**: `src/singularity.rs`, `src/reservoir.rs`, `src/framework.rs`, `src/persistence.rs`
**Hyperdim**: `src/hyperdim.rs`, `src/bundle.rs`
**Retrieval**: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`, `src/singularity_retrieval.rs`
**Bridge**: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
**CLI**: `src/cli/`
**State**: `plans/GOAP_STATE.md`, `plans/ACTIONS.md`

## Skills (27 Total)
**Core**: adr-creation, benchmarking-perf, debugging-reservoir, dist-channel-selection, drawio, git-workflow, github-ci-guardrails, goap-planning, memory-lifecycle-verification, npm-trusted-publishers, release-management, rust-development, testing-validation, turso-memory-verification
**Swarm**: analysis-swarm, swarm-advanced-features, swarm-observability
**Workflow**: learn, shell-script-quality, task-decomposition
**Automation**: jules-orchestration, self-fix-loop, skill-creator, skill-evaluator, skill-memory-internal
**TRIZ**: triz-analysis, triz-solver
## External References
- `@agents-docs/hard-constraints.md` — LOC limits, spectral radius
- `@agents-docs/accuracy-guardrails.md` — API verification, crate vetting
- `@agents-docs/quick-reference.md` — Unique commands not covered by skills
- `@agents-docs/self-learning-patterns.md` — Curated patterns; see also `@progress/LEARNINGS.md`
