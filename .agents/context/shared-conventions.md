# Shared Cross-Repository Conventions

Cross-repository conventions for `d-o-hub` organization projects (including `rust-2026-template` and derived crates such as `chaotic_semantic_memory`).

---

## Ownership & Conflict Resolution

- **Local Ownership**: Repository-specific implementation, validation rules, domain logic, and local skills reside in each repository's `@AGENTS.md` and local `.agents/skills/`.
- **Shared Context Scope**: This context document contains *only* conventions verified to apply across all d-o-hub repositories.
- **Conflict Resolution**: When a shared convention conflicts with a repository's local rules, **the local repository's safety and quality contract wins**. Document the conflict in local learnings rather than blindly syncing or overriding local standards.

---

## Canonical Instructions & Validation Discovery

Agents working across repositories can discover canonical entrypoints without scanning all skill trees:

- **Canonical Instructions**: Read `@AGENTS.md` and `@HARNESS.md` at the repository root.
- **Validation Commands**: Run the repository's standard validation entrypoint:
  - CSM: `./scripts/harness-check.sh all` and `./scripts/validate.sh`
  - Template: `scripts/quality-gates.sh` or `make test`

---

## Branch & PR Hygiene

- **Protected Main Branch**: Never commit or push directly to `main`. Create a feature branch off latest `main`.
- **Branch Naming**: Use `<type>/<scope>-<description>` (e.g., `feat/retrieval-bm25`, `fix/persistence-fk`).
- **Conventional Commits**: PR titles and commit messages must follow Conventional Commits format (`<type>(<scope>): <description>`).
- **Up-to-Date Rebase Before Merge**: Every PR must be rebased against current `main`, pass CI on that head, and be merged cleanly.
- **Squash Merge**: Prefer squash-and-merge with a clear, conventional commit title.

---

## Evidence-Based CI Claims

- **Verifiable Proof Required**: All assertions regarding performance gains, benchmark improvements, or CI fixes MUST include verifiable evidence (Criterion benchmark output, test logs, or recorded artifacts) in the PR description.
- **No Gate Evasion**: Changing commit types (e.g., `perf` to `refactor`) to bypass PR evidence requirements or claiming unverified performance gains is strictly prohibited.

---

## Issue & PR Scope

- **Atomic PRs**: Each PR should address a single focused task or issue scope.
- **Traceability**: Reference corresponding issues or ADR rationale (e.g., `@plans/adr/0090-harness-engineering-template-alignment.md`).
- **No Skill Duplication**: Do not copy `SKILL.md` trees or re-create local skill catalogs in shared context files.
