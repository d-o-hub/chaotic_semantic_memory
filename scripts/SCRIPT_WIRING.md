# Script wiring exceptions (referenced-by gate)

Reviewed assertions for `scripts/` entries whose wiring is an *operator path*
rather than a CI/validate caller or fixture. `scripts/gen_script_catalog.py
--check` (run by `scripts/check-script-catalog.sh` and `scripts/validate.sh`)
fails when an operator-only script has no row here, and when a row goes stale
(script wired or gone). Every row must cite evidence that can be re-derived:
a tracked document, an agent skill, or a fixture that invokes the script.

| Script | Wired by (evidence) |
|---|---|
| `scripts/ai-commit.sh` | documented operator command — `agents-docs/quick-reference.md` |
| `scripts/coverage-report.sh` | canonical test inventory: `AGENTS.md` (step 12 and Key Invariants name `scripts/coverage-report.sh inventory` as the re-derivation command for stored counters) |
| `scripts/pre-commit.sh` | full local pre-commit path documented in `HARNESS.md` (`.githooks/pre-commit` is the installed inline gate; this script is the operators' standalone entry) |
| `scripts/propagate-version.sh` | version-sync operator command documented in `agents-docs/quick-reference.md` |
| `scripts/release-manager.sh` | release-flow operator command documented in `.agents/skills/release-management/SKILL.md` |
| `scripts/release-no-verdict.sh` | invoked by the `scripts/test-release-verdict.sh` fixture (run by `scripts/validate.sh`); historical helper for the retired `no-verdict` release job, kept so its verdict taxonomy stays tested |
| `scripts/self-fix-loop.sh` | invoked by the `scripts/test-self-fix-loop-safety.sh` fixture (run by `scripts/validate.sh`); operator automation documented in `.agents/skills/self-fix-loop/SKILL.md` |
| `scripts/setup-hooks.sh` | invoked by the `scripts/test-hook-bootstrap.sh` fixture (run by `scripts/validate.sh`) |
| `scripts/skills-suggest.sh` | documented operator command — `agents-docs/quick-reference.md` |
| `scripts/validate-low-memory.sh` | documented — `CONTRIBUTING.md`, `agents-docs/quick-reference.md`, `.agents/skills/testing-validation/SKILL.md` |
| `scripts/validate-skills.sh` | documented operator command — `agents-docs/quick-reference.md` |
| `scripts/verify-memory-roundtrip.sh` | documented — `.agents/skills/turso-memory-verification/SKILL.md` |
| `scripts/wasm-evidence.sh` | ADR-0095 Tier-3 evidence command for the npm artifact; the tracked evidence it produced (`plans/evidence/wasm_2026_09_21/README.md`) records it |
