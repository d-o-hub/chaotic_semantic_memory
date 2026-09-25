# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

---

## Workflow (REQUIRED for Every Session)

### Phase 1: Context Load (WHAT)
1. **Read state files first**: `@plans/GOAP_STATE.md` (canonical state) and `@plans/ACTIONS.md` (active queue).
2. **Review uncommitted changes**: `git status --short && git diff HEAD`. Scope out or commit unrelated diffs first.
3. **LOC gate pre-check**: `find src crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | sort -rn | head -20` (all source files must be ≤ 500 LOC).
4. **Parity & CI check**: `./scripts/check-adr-parity.sh` (ADR parity), `gh run list --workflow=ci.yml --limit 3`, and check conflicts via `scripts/pr-triage.sh` (or `gh pr list --state open --json number,mergeable --jq '.[] | select(.mergeable == "CONFLICTING")'`).
5. **Roast before implement or merge (MANDATORY)**: every GitHub PR and issue is reviewed and roasted **before** implementing it or merging it — never take a PR/issue at face value. Run the `pr-roast-triage` skill: verify CI truth from annotations (not badges), detect duplicates, and apply the roast rubric (atomicity, title/body honesty, commitlint scope, preserved rationale comments, additive-only `deny.toml`, measured perf claims, no lockfile/`export.json` noise). A PR with no demonstrated impact is **closed as no-op with a roast comment**, not merged. Record the verdict in `plans/PR_ROAST_<YYYY_MM_DD>.md`, update `progress/PROGRESS.md` + `progress/LEARNINGS.md`, and distill the reusable lesson into `.agents/skills/` (update/compact an existing skill rather than adding a near-duplicate). See `skill://pr-roast-triage`.
6. **Verify binary truth**: Confirm behavior against `./target/debug/csm`, never against a stale global install.

### Phase 2: Planning (WHY)
7. **Plan before implementing**: For tasks with 3+ steps, map affected files/crates into `plans/`. Use `triz-analysis` for trade-offs and `task-decomposition` for swarms.
8. **Pre-flight dependency graph**: Analyze crate dependencies before splitting work across multiple PRs.

### Phase 3: Implementation (HOW)
9. **Precision editing**: Read before editing. Preserve comments and docstrings. Child module extraction (e.g. `hyperdim_binary_serde.rs`) is required over comment stripping when approaching 500 LOC.
10. **Perf claims require evidence**: Any PR with perf claims MUST attach Criterion benchmark numbers or flamegraphs via `benchmarking-perf`.
11. **Validation gates**: Run `./scripts/validate.sh` (fmt, clippy `-D warnings`, test, deny, ADR parity). If touching CLI, run `cargo test --test cli_parity --features cli`.
12. **Update state files**:
   - Update `plans/GOAP_STATE.md`: `action_last_completed` (MUST appear exactly once, last key), module LOC, test counts.
   - Update `plans/ACTIONS.md`: remove completed action, update status.
   - Append learnings to `progress/LEARNINGS.md` and progress to `progress/PROGRESS.md`.

### Phase 4: Compound Engineering
13. **Encode corrections**: Fix the immediate issue, then encode a rule in `AGENTS.md` or `agents-docs/` to prevent recurrence.

### Phase 5: Atomic Commit, PR & CI Gate
14. **Branch first**: `main` is protected; always create a branch (`git checkout -b <type>/<scope>-<desc>`).
15. **Commitlint validation**: Full range `npx commitlint --from origin/main --to HEAD --verbose`. Scopes are strictly defined in `commitlint.config.cjs`.
16. **Push and create PR**: Push branch, create PR with clear description. If resolving child issues, list `Fixes #<id>` for each issue.
17. **Roast verdict required before merge (MANDATORY)**: a PR may not merge until it has been reviewed and roasted per Phase 1 step 5 — CI green is necessary, not sufficient. Record the verdict (`plans/PR_ROAST_<date>.md`). On a no-impact PR: post the roast comment, close it, then update `progress/` and distill the lesson into `.agents/skills/`. Do not silently re-implement a closed PR's idea; a resubmission needs the evidence the roast demanded.
18. **CI truth & merge (one at a time, always current)**: **NEVER use `gh pr merge --auto`** — merge explicitly. Every PR MUST be **up to date with the latest `main`** at merge time: `git fetch origin main`, rebase the PR branch, push, wait for CI to go green on that new head, re-check `mergeable`/`mergeStateStatus`, and only then `gh pr merge <n> --squash --delete-branch`. A green CI on a stale branch is not merge evidence — it never ran against the tree that would land. Each merge moves `main`, so after every merge repeat the rebase → CI → merge cycle for the next PR; never batch merges on stale heads and never merge while another merge is in flight.

---

## 8 Core Rules

1. **Always read before editing** — Never guess file contents.
2. **Stay under context limits** — Each instruction must earn its place. Keep AGENTS.md ≤ 200 LOC.
3. **Deterministic gates** — `./scripts/validate.sh` and `./scripts/harness-check.sh all` are mandatory.
4. **Use `@imports` for modularity** — Reference docs via `@path/to/file` syntax.
5. **Plan before implementing** — Map dependencies before touching 3+ files.
6. **Encode errors immediately** — Every bug or workflow friction becomes a documented rule.
7. **Reference, don't duplicate** — Point to source files/ADRs, do not duplicate content.
8. **Never push directly to `main`** — Branch → commit → PR → verify green CI → squash-merge. Every PR MUST be merged from a head that is up to date with the latest `main` (rebase → re-run CI → merge, one PR at a time); **NEVER** `gh pr merge --auto`.
9. **Roast before implement or merge** — Every GitHub PR/issue gets reviewed and roasted before you act on it. No demonstrated impact → close as no-op with the roast comment, then record the verdict, update `progress/`, and distill the lesson into `.agents/skills/`. CI green ≠ approved.

---

## Key Invariants & Standards

- **Hard Constraints**: Spectral radius in [0.9, 1.1], all files ≤ 500 LOC, test:source ratio ≥ 90%. Details: `@agents-docs/hard-constraints.md`.
- **Coding Standards & DeepSource Parity**: Direct struct construction in `Default::default()`; `.map_or()` / `.is_some_and()` over `.map().unwrap_or()`; `unwrap`/`expect`/`panic` forbidden in lib without justification. Details: `@agents-docs/coding-standards.md`.
- **Release Safety**: Never release with failing CI; synchronized `Cargo.lock`; verify crate ownership. Details: `@agents-docs/release-safety.md`.
- **GitHub API Fallback**: Default `gh issue view` and `gh pr edit` fail on deprecated `projectCards`. Query `--json` fields explicitly or use `gh api -X PATCH repos/.../pulls/<id>`.

---

## Key Files
- **Core**: `src/singularity.rs`, `src/reservoir.rs`, `src/framework.rs`, `src/persistence.rs`
- **Bridge**: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
- **Retrieval**: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`, `src/singularity_retrieval.rs`
- **CLI & MCP**: `src/cli/commands/*.rs`, `src/mcp/tools.rs`
- **State & Plans**: `plans/GOAP_STATE.md`, `plans/ACTIONS.md`

---

## Skills (33 Total)
- **Core**: `rust-development`, `testing-validation`, `goap-planning`, `goap-orchestrator`, `adr-creation`, `github-ci-guardrails`, `git-workflow`, `release-management`, `dist-channel-selection`, `benchmarking-perf`, `debugging-reservoir`, `skill-memory-internal`, `memory-lifecycle-verification`, `turso-memory-verification`, `drawio`, `npm-trusted-publishers`
- **Swarm**: `swarm-testing-quality`, `swarm-performance`, `swarm-observability`, `swarm-advanced-features`, `analysis-swarm`
- **Workflow**: `learn`, `task-decomposition`, `shell-script-quality`, `jules-orchestration`, `pr-roast-triage`
- **Automation**: `self-fix-loop`, `iterative-refinement`, `skill-creator`, `skill-evaluator`, `codacy`
- **TRIZ**: `triz-analysis`, `triz-solver`

---

## External References
- `@agents-docs/hard-constraints.md` — LOC limits, spectral radius, invariants
- `@agents-docs/coding-standards.md` — DeepSource parity, clippy & lint policies
- `@agents-docs/release-safety.md` — Release checklist, channels, recovery
- `@agents-docs/accuracy-guardrails.md` — API verification
- `@agents-docs/quick-reference.md` — Commands and cheat sheet
- `@agents-docs/self-learning-patterns.md` — Compound engineering
