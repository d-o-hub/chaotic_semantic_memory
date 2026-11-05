# ADR-0039: Release Engineering

## Status

Accepted (backfilled 2026-05-01) - Wave 11 Complete

> Updated 2026-11-05: the crates.io deliverable below was **claimed, never delivered**.
> Publishing ran on a long-lived `CARGO_REGISTRY_TOKEN` that expired (#967: nine 403s), and
> the "No long-lived API tokens" consequence was aspirational. OIDC Trusted Publishing is
> actually delivered by **ADR-0104**, which had to move crates.io publishing out of
> `release.yml` because crates.io refuses tokens minted from the `workflow_run` event that
> triggers this workflow.

## Context

Release process was manual and error-prone:
- No automated versioning
- Manual crates.io publishing with API tokens
- No npm provenance for WASM
- Documentation not auto-deployed

## Decision

Implement **automated release engineering**.

**Deliverables:**
- semantic-release for automated versioning
- Trusted Publishing for crates.io (OIDC-based, no tokens) — *not delivered as written; see ADR-0104 (2026-11-05)*
- npm provenance for WASM bindings (--provenance flag)
- mdBook for GitHub Pages documentation
- CLI usage examples and documentation

## Consequences

### Positive
- Zero-touch releases
- No long-lived API tokens (security) — true for crates.io only from ADR-0104 onward
- Supply chain verification (provenance)
- Auto-deployed documentation
- Trusted Publishing best practices

### Negative
- Release workflow complexity
- OIDC configuration required
- mdBook maintenance
- Version sync automation needed

## Implementation

- Files: .github/workflows/release.yml, npm-publish.yml, pages.yml
- Skill: .agents/skills/release-management/
- Book: book/src/*.md

## Sources

- ACTIONS.md lines 1633-1854 (Phase 25 actions)
- W11_Release_Engineering_Complete.md handoff
- ADR_REGISTRY.md: Wave 11 Active ADRs