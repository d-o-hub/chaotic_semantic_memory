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

2. **Review ALL uncommitted changes** — Never start implementation without knowing the full scope:
   ```bash
   git status --short           # List ALL modified/untracked files
   git diff HEAD                # Review content of pending changes
   ```
   - If unrelated changes exist, either commit them first or explicitly scope them out
   - Document which pending changes are intentionally excluded from this session

3. **Understand the codebase structure** — Know where files live before editing:
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

4. **Plan before implementing** — For non-trivial tasks (3+ steps):
   - Explore codebase before proposing changes
   - Identify affected files and dependencies
   - Document approach in `plans/` directory
   - Get user approval before implementation
   - **TRIZ Integration**: Use `triz-analysis` skill for architectural decisions
   - **Problem Solving**: Use `triz-solver` skill when stuck on complex problems

5. **Use parallel execution for complex changes** — For multi-file tasks:
   - Create task list for each subtask
   - Spawn specialized workers with clear prompts
   - Assign tasks and monitor progress
   - Clean up resources after completion

### Phase 3: Implementation (HOW)

6. **Edit files with precision** — Never bulk-edit without reading first:
   - Read before editing — understand existing code
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

12. **Push and monitor CI** — Watch workflow until completion:
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

15. **Protected branches require PR** — Branch protection enforced:
    - `main` requires pull request
    - Verify CI passes before merge

---

## Session Checklist

Before starting any task, verify:
- [ ] GOAP_STATE.md loaded — know current state
- [ ] ALL uncommitted changes reviewed via `git status --short`
- [ ] Hard constraints understood — LOC <=500, spectral radius [0.9, 1.1]
- [ ] CI baseline confirmed via `gh run list`

Before completing any task, verify:
- [ ] All validation gates pass (check, test, fmt, clippy)
- [ ] CI workflow passes
- [ ] GitHub Actions warnings/issues checked via `gh run view`
- [ ] Pre-existing warnings fixed (not just new issues)
- [ ] GOAP_STATE.md updated with `action_last_completed`
- [ ] Learnings captured if new patterns discovered

---

## 7 Core Rules

1. **Always read before editing** — Never guess file contents.

2. **Stay under context limits** — Each instruction must earn its place.

3. **Hooks for deterministic enforcement** — Validation gates are mandatory.

4. **Use `@imports` for modularity** — Reference files via `@path/to/file` syntax.

5. **Plan before implementing** — For tasks with 3+ steps.

6. **Update monthly, encode errors immediately** — Every correction becomes a rule.

7. **Reference, don't duplicate** — Point to source files, don't restate contents.

---

## Hard Constraints
See: [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md)

---

## Release Safety Requirements

**CRITICAL: Never release with failing CI. The release workflow now has a guardrail that waits for CI to pass.**

### Artifact Selection (REQUIRED)

Before validating, installing, or publishing, identify the correct channel:
- **Rust Library:** `chaotic_semantic_memory` (crates.io / cargo)
- **JS/WASM Library:** `@d-o-hub/chaotic_semantic_memory` (npm WASM)
- **CLI Tool:** `@d-o-hub/csm` (npm CLI)

Refer to the `dist-channel-selection` skill for canonical commands.

### Pre-Release Checklist (MANDATORY)

1. **Verify CI passes on all platforms**:
   ```bash
   gh run list --workflow=ci.yml --limit 3
   gh run view --log  # Check all jobs: macos-arm64, windows-x64, linux
   ```

2. **Ensure Cargo.lock is synchronized**:
   ```bash
   cargo build --release  # Regenerates Cargo.lock after version bump
   git add Cargo.lock     # Must be committed with version changes
   ```

3. **Check existing releases**:
   ```bash
   gh release list --limit 5
   gh release view --json tagName,isLatest
   ```

4. **Validate changelog entry exists**:
   ```bash
   grep -q "^## \[${VERSION}\]" CHANGELOG.md
   ```

### Version Bump Workflow

1. Update `Cargo.toml` version
2. Update `wasm/package.json` version
3. Update `CHANGELOG.md` with new section
4. Run `cargo build --release` to sync Cargo.lock
5. Commit all version files together (atomic)
6. Push and wait for CI to pass
7. Only then create tag/release

### Platform-Specific Considerations

- **macOS arm64**: NEON SIMD intrinsics require explicit unsafe blocks
- **Windows x64**: CI uses `--locked` flag, Cargo.lock must match Cargo.toml
- **WASM**: Size gate checks library (~870KB), not CLI binary (~5KB)

### Reference Files

- `.github/workflows/release.yml` — Has `wait-for-ci` guardrail job
- `.agents/skills/release-management/` — Full release skill
- `scripts/validate.sh` — Pre-commit validation gates

---
## Key Files
**Core**: `src/singularity.rs`, `src/reservoir.rs`, `src/reservoir_inertial.rs`, `src/framework.rs`, `src/persistence.rs`
**Bridge**: `src/semantic_bridge.rs`, `src/bridge_retrieval.rs`
**Retrieval**: `src/retrieval/bm25.rs`, `src/retrieval/hybrid.rs`, `src/singularity_retrieval.rs`
**CLI**: `src/cli/commands/query.rs`, `src/cli/commands/index_dir.rs`
**State**: `plans/GOAP_STATE.md`, `plans/ACTIONS.md`

## Skills (30 Total)
**Core**: `rust-development`, `testing-validation`, `goap-planning`, `adr-creation`, `github-ci-guardrails`, `git-workflow`, `release-management`, `dist-channel-selection`, `benchmarking-perf`, `debugging-reservoir`, `skill-memory-internal`, `memory-lifecycle-verification`, `turso-memory-verification`, `drawio`, `npm-trusted-publishers`

**Swarm**: `swarm-testing-quality`, `swarm-performance`, `swarm-observability`, `swarm-advanced-features`, `analysis-swarm`

**Workflow**: `learn`, `task-decomposition`, `shell-script-quality`

**Automation**: `self-fix-loop`, `iterative-refinement`, `skill-creator`, `skill-evaluator`

**TRIZ**: `triz-analysis`, `triz-solver`

## External References
- [agents-docs/hard-constraints.md](agents-docs/hard-constraints.md) — LOC limits, spectral radius
- [agents-docs/accuracy-guardrails.md](agents-docs/accuracy-guardrails.md) — API verification
- [agents-docs/quick-reference.md](agents-docs/quick-reference.md) — Commands
- [agents-docs/self-learning-patterns.md](agents-docs/self-learning-patterns.md) — Compound engineering