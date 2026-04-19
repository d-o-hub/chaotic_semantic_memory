# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

---

## Workflow (REQUIRED for Every Session)

**This workflow MUST be followed for every coding task. Skipping steps causes regressions.**

### Phase 1: Context Load (WHAT)

1. **Read state files first** — Load current world state before any work:
   - `@plans/GOAP_STATE.md` — Current world state, completed phases, module LOC
   - `@plans/ACTIONS.md` — Queued actions and their preconditions

2. **Understand the codebase structure** — Know where files live before editing:
   - Core modules: `src/singularity.rs`, `src/reservoir.rs`, `src/framework.rs`
   - Persistence: `src/persistence.rs`, `src/persistence_ops.rs`
   - Retrieval: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`
   - Bridge: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
   - CLI: `src/cli/commands/*.rs`

3. **Check CI status** — Verify baseline before changes:
   ```bash
   gh run list --workflow=ci.yml --limit 3
   ```

### Phase 2: Planning (WHY)

4. **For non-trivial tasks (3+ steps), use Plan Mode** — Enter plan mode via `EnterPlanMode` tool:
   - Explore codebase before proposing changes
   - Identify affected files and dependencies
   - Document approach in `plans/` directory
   - Get user approval before implementation

5. **For complex multi-file changes, spawn teammates** — Use `TeamCreate` + `Agent` for parallel work:
   - Create task list via `TaskCreate`
   - Spawn specialized agents with clear prompts
   - Assign tasks via `TaskUpdate(owner=...)`
   - Monitor via idle notifications
   - Clean up with shutdown requests and `TeamDelete`

### Phase 3: Implementation (HOW)

6. **Edit files with precision** — Never bulk-edit without reading first:
   - Use `Read` before `Edit` — understand existing code
   - Match existing style, naming, patterns
   - Preserve comments and docstrings unless explicitly removing

7. **Run validation gates after changes** — Verify before proceeding:
   ```bash
   cargo check --quiet                      # Compile check
   cargo test --all-features --quiet        # Unit + integration tests
   cargo fmt --check --quiet                # Format check
   cargo clippy --quiet -- -D warnings      # Lint check
   ```

8. **Update state after completion** — Record what changed:
   - Update `GOAP_STATE.md`: `action_last_completed`, module LOC, test counts
   - Add learnings to `progress/LEARNINGS.md` if new patterns discovered

### Phase 4: Verification (Compound Engineering)

9. **Run full validation before claiming completion**:
   ```bash
   ./scripts/validate.sh                    # All gates in one command
   ```

10. **If errors occur, encode corrections** — Compound engineering principle:
    - Fix the immediate error
    - Add rule/constraint to prevent recurrence
    - Update AGENTS.md or hard-constraints.md if systemic

### Phase 5: Atomic Commit & CI Gate (GOAP Orchestration)

11. **Atomic commits** — One logical change per commit, never mix unrelated changes:
    ```bash
    git add src/singularity.rs src/singularity_cache.rs
    git commit -m "feat(singularity): add similarity cache"
    ```

12. **Push to branch and monitor CI** — Use `github-ci-guardrails` skill:
    ```bash
    git push origin <branch>
    gh run watch --exit-status
    gh run list --workflow=ci.yml --limit 1
    ```

13. **Fix ALL issues (including pre-existing)** — CI must pass completely:
    - New failures: Fix immediately
    - Pre-existing warnings: Fix before claiming completion
    - Use `goap-planning` skill to track fix actions in GOAP_STATE
    - Update `action_last_completed` and `world_state` after each fix

14. **Document in GOAP_STATE** — Record completion state:
    ```yaml
    world_state:
      action_last_completed: <action_name>
      ci_all_checks_passed: true
      tests_count: <new_count>
    ```

15. **Push to main requires PR** — Branch protection enforced:
    ```bash
    gh pr create --title "type(scope): description" --body "$(cat <<'EOF'
    ## Summary
    - Change 1

    ## Test plan
    - [ ] cargo test --all-features
    - [ ] CI passes
    EOF
    )"
    gh pr merge --squash --delete-branch
    ```

---

## Session Checklist

Before starting any task, verify:
- [ ] GOAP_STATE.md loaded — know current state
- [ ] Hard constraints understood — LOC <=500, spectral radius [0.9, 1.1]
- [ ] CI baseline confirmed via `gh run list --workflow=ci.yml --limit 3`

Before completing any task, verify:
- [ ] All validation gates pass (check, test, fmt, clippy, LOC)
- [ ] CI workflow passes — `gh run watch --exit-status` exits 0
- [ ] Pre-existing warnings fixed (not just new issues)
- [ ] GOAP_STATE.md updated with `action_last_completed`
- [ ] Learnings captured if new patterns discovered

---

## 7 Core Rules

1. **Always read before editing** — Use `Read` tool before `Edit`. Never guess file contents.

2. **Stay under context limits** — AGENTS.md is advisory (~160 instructions reliably followed). Each line must earn its place.

3. **Hooks for deterministic enforcement** — Validation gates (test, clippy, LOC) are mandatory.

4. **Use `@imports` for modularity** — Reference files via `@path/to/file` syntax.

5. **Plan mode for 3+ step tasks** — Use `EnterPlanMode` before implementation.

6. **Update monthly, encode errors immediately** — Every correction becomes a rule.

7. **Reference, don't duplicate** — Point to `Cargo.toml`, `context.yaml`, `GOAP_STATE.md`.

---

## Hard Constraints
See: [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md)

## Key Files
**Core**: `src/singularity.rs`, `src/reservoir.rs`, `src/framework.rs`, `src/persistence.rs`
**Bridge**: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
**Retrieval**: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`
**CLI**: `src/cli/commands/query.rs`, `src/cli/commands/index_dir.rs`
**State**: `plans/GOAP_STATE.md`, `plans/ACTIONS.md`

## Skills (19 Total)
**Core**: `rust-development`, `testing-validation`, `goap-planning`, `adr-creation`, `github-ci-guardrails`, `git-workflow`, `release-management`, `benchmarking-perf`, `debugging-reservoir`, `skill-memory-internal`, `memory-lifecycle-verification`, `turso-memory-verification`, `drawio`

**Swarm**: `swarm-testing-quality`, `swarm-performance`, `swarm-observability`, `swarm-advanced-features`, `analysis-swarm`

## External References
- [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md) — LOC limits, spectral radius
- [agents-docs/accuracy-guardrails.md](agents-docs/accuracy-guardrails.md) — API verification
- [agents-docs/quick-reference.md](agents-docs/quick-reference.md) — Commands
- [agents-docs/self-learning-patterns.md](agents-docs/self-learning-patterns.md) — Compound engineering