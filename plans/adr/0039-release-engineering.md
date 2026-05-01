# ADR-0039: Release Engineering

## Status

Accepted (backfilled 2026-05-01) - Wave 11 Complete

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
- Trusted Publishing for crates.io (OIDC-based, no tokens)
- npm provenance for WASM bindings (--provenance flag)
- mdBook for GitHub Pages documentation
- CLI usage examples and documentation

## Consequences

### Positive
- Zero-touch releases
- No long-lived API tokens (security)
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