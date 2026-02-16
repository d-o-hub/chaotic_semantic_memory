# AGENTS.md - Chaotic Semantic Memory

## Mission
Build and maintain `chaotic_semantic_memory` as a production Rust crate for AI memory systems.

## Hard Constraints
- Source files: `<= 500 LOC` each.
- `SKILL.md` (.agents/skills/ folder): `<= 250 LOC`; move detail to `references/`, `scripts/`, or `assets/`.
- Use `libsql` (never `turso-client`).
- Use Tokio async/await for I/O.
- Use Rayon for CPU parallelism.
- All public APIs return `Result<T, Error>`.
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- WASM threading paths must be gated with `#[cfg(not(target_arch = "wasm32"))]`.

## Skill Rules
- Keep frontmatter to `name` + `description` only.
- Put trigger conditions in `description`.
- Keep references one level deep and link each from `SKILL.md`.
- Avoid duplicated guidance between `SKILL.md` and `references/*`.

## Skills
- `adr-creation`: `.agents/skills/adr-creation/SKILL.md`
- `goap-planning`: `.agents/skills/goap-planning/SKILL.md`
- `rust-development`: `.agents/skills/rust-development/SKILL.md`
- `testing-validation`: `.agents/skills/testing-validation/SKILL.md`
- `github-ci-guardrails`: `.agents/skills/github-ci-guardrails/SKILL.md`

## Accuracy Guardrails
- Do not assume crate existence/version; verify.
- When uncertain on modern Rust practice, verify with web research.
- If a decision changes architecture, write/update ADR.
- Prefer exact, testable instructions over high-level advice.

## Git + CI Source of Truth
- Use atomic commits: one logical change per commit.
- Before commit, run:
  - `cargo check`
  - `cargo test --all-features`
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
- Treat GitHub Actions as merge gate source of truth.
- Use `gh` CLI to verify checks for the target branch/PR:
  - `gh pr status`
  - `gh pr checks --watch`
  - `gh run list --branch <branch> --limit 5`
- Do not claim success until local checks and relevant GitHub checks pass.

## Performance Gate
- Run `cargo bench -- --baseline` before closing performance-sensitive work.
- Validate: `reservoir_step < 100us @ 50k nodes`.

## Learning Loop (RALPH)
After each iteration:
1. Record what worked in `Discovered Patterns`.
2. Record failures in `Gotchas & Warnings`.
3. Update module LOC counts.
4. Run test + bench gates.
5. Commit message format: `RALPH iteration N: [summary]`.

## Discovered Patterns
- read/update learnings in progress/LEARNINGS.md
- read/update progress in progress/PROGRESS.md