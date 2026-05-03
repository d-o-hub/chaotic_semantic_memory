# ADR-0050: npm Node.js 24 + Token Fallback

## Status

Accepted (backfilled 2026-05-01) - 2026-02-28

## Context

npm publishing failures:
- OIDC trusted publishing not working
- "404 Not Found" / "Access token expired" errors
- Node.js 22 npm version too old for OIDC
- Package not found during publish

## Decision

Fix **npm publishing with Node.js 24 + token fallback**.

**Deliverables:**
- Node.js version: 24 (npm v11+ required for OIDC)
- OIDC first, fall back to NPM_TOKEN secret if provided
- Manual first publish required before OIDC
- Trusted Publisher configuration in npm UI

## Consequences

### Positive
- OIDC works with Node.js 24
- Fallback for manual token
- Automated publishing restored
- Trusted Publishing configured

### Negative
- Node.js 24 requirement
- Manual first publish still needed
- Token fallback less secure
- Configuration complexity

## Implementation

- File: .github/workflows/npm-publish.yml
- Node: node-version: '24'
- Fallback: NPM_TOKEN secret support
- OIDC: Trusted Publisher in npm UI

## Sources

- Git: fix(ci): upgrade Node.js to 24 (ADR-0050) (2026-02-28)
- ADR_REGISTRY.md: npm Node.js 24 + Token Fallback
- ACTIONS.md lines 1818-1833