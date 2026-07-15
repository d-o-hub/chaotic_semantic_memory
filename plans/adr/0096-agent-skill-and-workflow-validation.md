# ADR-0096: Agent Skill and Workflow Validation

## Status

Proposed (2026-07-14)

## Context and Problem Statement

The canonical skill inventory contains 32 `SKILL.md` files, while planning and agent documentation record 29 or 30. The current format validator reports all skills valid but does not enforce the documented 250-line hard limit, quoted-description policy, required sections, or local-reference resolution. `.agents/skills/release-management/SKILL.md` is 294 lines and contradicts the repository's protected-main and automatic-tag workflow.

Several skill-local validation scripts infer success by grepping command output rather than preserving Cargo exit status. Hook installation, workflow validation, version checks, and link/reference checks are fragmented and sometimes disagree. Only a small minority of skills have behavioral evaluation artifacts.

Planning is part of the same agent control plane. `GOAP_STATE.md` has duplicate canonical keys and stale current-state claims; `ACTIONS.md` has a duplicate action name and an action marked delegated after its implementation landed. Very large active state files make stale history look executable.

## Decision Drivers

- Agent instructions and validators affect release and code safety.
- Validation must fail closed and be fixture-tested.
- Skill hard constraints must be executable, not prose-only.
- Catalogs and counts should be generated from disk.
- Critical workflow skills need behavioral evaluation, not only syntax checks.
- Historical plans must remain auditable without polluting active state.

## Considered Options

1. Keep independent validators and manually reconcile drift.
2. Enforce only frontmatter/LOC statically.
3. Create one canonical validation library/entry point plus critical-skill evaluations.
4. Remove repository skills and rely only on global agent instructions.

## Decision Outcome

Chosen option: **one fail-closed validation graph, generated inventory, and risk-based behavioral evaluations; active plans are linted and history is archived non-destructively**.

### Canonical skill validator

The validator must:

- discover `.agents/skills/*/SKILL.md` and generate the catalog/count;
- parse YAML frontmatter rather than approximate it with grep;
- enforce directory/name parity, the chosen description policy, required purpose/trigger content, and `SKILL.md <=250` lines;
- resolve repository-root, skill-relative, and external/placeholder links with explicit semantics;
- reject absolute local symlinks and broken references;
- check referenced scripts for executable status and shell syntax;
- emit machine-readable findings and nonzero exit on any violation.

Repository docs consume or verify the generated catalog instead of maintaining independent totals.

### Fail-closed command execution

Canonical quality scripts invoke commands directly and preserve exit status. Output filtering occurs after status capture and cannot turn failure into success. Negative fixtures simulate failures for check, test, fmt, clippy, doc, deny, fuzz build, and link validation. Each command runs once.

Skill-local scripts delegate to the canonical entry point or implement only skill-specific checks; near-duplicate generic validators are removed.

### Behavioral evaluations

At minimum, critical skills receive basic, complex, edge, and read-only scenarios:

- `goap-planning`;
- `testing-validation`;
- `git-workflow`;
- `release-management`;
- `skill-memory-internal`.

Each has positive triggers, negative triggers, expected tool/file boundaries, and machine-checkable outcomes. Initial gate: at least 19/20 scenarios pass and all deterministic fields are consistent. Other skills are added by risk and change frequency.

### Release and hook policy

- Release skill uses branch -> PR -> required CI -> merge; no routine direct-main push.
- It documents one trigger matching `.github/workflows/release.yml`; destructive recovery commands are conditional and approval-gated.
- One hook bootstrap installs and verifies the canonical pre-commit, commit-msg, and pre-push set.
- Local full validation and required CI share the same gate graph; fast hooks may run an explicitly documented subset.

### Plan lint and compaction

Plan validation rejects duplicate top-level world-state keys, duplicate action names, invalid status values, impossible status/effect combinations, and multiple `action_last_completed` keys.

Compaction is non-destructive:

1. define active-vs-historical criteria;
2. audit inbound references;
3. write an archive manifest and redirects/index;
4. move only immutable completed material;
5. validate all links and planning scripts.

The user-owned `plans/RECOMMENDATIONS_2026_07_14.md` is not moved or rewritten without explicit approval.

## Positive Consequences

- A green validator becomes meaningful.
- Skill constraints and release safety are continuously enforced.
- Catalog drift disappears.
- Critical instructions are tested against actual outcomes.
- Active GOAP state becomes smaller and unambiguous without losing history.

## Negative Consequences

- YAML/link parsing and fixtures add maintenance.
- Some existing skills and references will fail when the stricter gate is introduced.
- Behavioral evaluations consume CI/runtime budget.
- Archive work must proceed slowly to preserve references.

## Pros and Cons of the Options

### Manual reconciliation

- Good, because no tooling is required.
- Bad, because the current 29/30/32 drift and false-success scripts prove it does not scale.

### Static-only validation

- Good, because it is fast and deterministic.
- Bad, because valid syntax does not prove safe release or validation behavior.

### Canonical validator plus risk-based evals

- Good, because syntax, references, execution, and outcomes are covered.
- Good, because expensive evals focus on critical skills.
- Bad, because fixtures and scoring need ownership.

### Remove repository skills

- Good, because duplication disappears.
- Bad, because repository-specific release, memory, and architecture knowledge would be lost.

## TRIZ Rationale

- **Taking out:** keep concise skill entry points and move detail to checked references.
- **Segmentation:** static conformance and behavioral effectiveness are separate gates.
- **Feedback:** validator fixtures encode each discovered failure mode.
- **Nested doll:** active plans reference indexed archives rather than embedding all history.

## Follow-up Actions

- `make_skill_validation_fail_closed`
- `align_release_skill_with_protected_workflow`
- `canonicalize_hooks_skill_refs_and_catalog`
- `complete_harness_missing_artifacts`
- `compact_active_plans_non_destructively`

## Acceptance Criteria

- Disk-derived inventory reports exactly 32 skills until intentionally changed.
- Every `SKILL.md` is `<=250` lines and all local references resolve.
- A simulated failure for each canonical command makes validation exit nonzero; each command executes once.
- Release guidance contains no routine direct-main push and matches the active workflow trigger/tag owner.
- Critical-skill evaluation score is at least 19/20 with deterministic-field consistency of 100%.
- A clean-clone hook test installs and verifies pre-commit, commit-msg, and pre-push.
- Plan lint rejects duplicate keys/action names and invalid statuses.
- Active-plan compaction preserves a manifest, inbound links, and the user-owned recommendations file.
