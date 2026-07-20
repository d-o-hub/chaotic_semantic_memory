# Plans Archive Manifest

**Archive date:** 2026-07-20  
**Policy:** Non-destructive (ADR-0096 / Wave 32 `compact_active_plans_non_destructively`)  
**Location:** `plans/.archive/2026-07-20-historical/`

Historical completed analysis, wave handoffs, and one-shot GOAP plans were moved out of the active `plans/` root so agents load a compact current state. Nothing was deleted.

## Active plan set (do not archive without re-audit)

| Path | Role |
|------|------|
| `plans/README.md` | Index of active planning docs |
| `plans/GOAP_STATE.md` | Canonical world state (YAML) |
| `plans/ACTIONS.md` | Action queue (complete + queued) |
| `plans/GOALS.md` | Primary / engineering goals |
| `plans/GOAP_ORCHESTRATOR.md` | Orchestrator runbook |
| `plans/GOAP_AUDIT_2026_07_14.md` | Wave 32 roadmap (still in progress) |
| `plans/RECOMMENDATIONS_2026_07_20.md` | Current recommendations (this analysis) |
| `plans/ADR_REGISTRY.md` | ADR index |
| `plans/adr/` | Architecture decision records |
| `plans/ARCHIVE_MANIFEST.md` | This file |

## Archived in `2026-07-20-historical/completed-goap/` (25 files)

| File | Why archived |
|------|----------------|
| `GAP_ANALYSIS_2026_04_30.md` | Superseded by later audits |
| `GAP_ANALYSIS_2026_06_26.md` | Wave 30 snapshot; complete |
| `GOAP_ANALYSIS_2026_04_25.md` | Historical analysis |
| `GOAP_CI_REMEDIATION_MUTATION_PR363.md` | One-shot CI fix complete |
| `GOAP_CI_REMEDIATION_PR356.md` | One-shot CI fix complete |
| `GOAP_PRE_EXISTING_ISSUES_PR356.md` | One-shot remediation complete |
| `VERIFICATION_2026_04_29.md` | Dated verification snapshot |
| `VERIFICATION_2026_04_30.md` | Dated verification snapshot |
| `WAVE_21_P0_COMPLETION.md` | Wave complete |
| `GOAP_CLI_EXAMPLES.md` | Implementation complete |
| `GOAP_CLIPPY_BEST_PRACTICES.md` | Implementation complete |
| `GOAP_COVERAGE_IMPROVEMENT.md` | Coverage gaps closed |
| `GOAP_LIFECYCLE_VERIFICATION_FOLLOWUP.md` | Follow-up complete / superseded |
| `GOAP_MAP_PAPER_ANALYSIS.md` | Research note; not active work |
| `GOAP_SKILL_MEMORY_HARDENING.md` | Hardening landed |
| `GOAP_SEMANTIC_BRIDGE.md` | Bridge shipped (ADR-0061) |
| `GOAP_DUCKDB_COMPANION_CRATE.md` | Companion crate shipped |
| `GOAP_WORKSPACE_COMPLETION.md` | Workspace extract snapshot |
| `GOAP_BENCHMARK_SUITE.md` | Suite exists; evidence work is in ACTIONS |
| `benchmark_optimization_actions.md` | Phase plan complete |
| `benchmark_optimization_plan.md` | Phase plan complete |
| `DEPENDABOT_ALERTS.md` | Snapshot; live source is GH Dependabot |
| `UNMAINTAINED_CRATES.md` | Snapshot; use `cargo deny` / deny.toml |
| `swarm_audit_github_2026.md` | Historical swarm audit |
| `SWARM_COORDINATION.md` | Historical coordination log |

## Archived in `2026-07-20-historical/handoffs/` (49 files)

All Wave W1–W20+ agent handoffs (`analysis_*`, `W*_*.md`, coordination notes).  
Restore path if needed: `plans/.archive/2026-07-20-historical/handoffs/<name>`.

## Redirect rule for agents

1. Prefer **active** files above for current work.
2. If a skill or doc links an archived path, resolve via this manifest.
3. Do **not** bulk-delete archives; keep git history + this directory.
4. New dated analyses go under `plans/` only while active; archive when complete.

## Reference audit notes

Inbound references to archived filenames may still appear in:

- `plans/GOAP_STATE.md` / `plans/ACTIONS.md` (historical notes — intentional)
- Old commit messages / PR bodies
- Skill or progress docs that cite wave handoffs

Those references remain valid via `plans/.archive/2026-07-20-historical/…`.
