# ADR-0050: npm Publishing - Node.js 24 + Token Fallback

## Status
**Accepted** - 2026-02-28

## Context and Problem Statement

The npm-publish.yml workflow was failing with:
- `404 Not Found` error when publishing
- `Access token expired` misleading message

**Root Cause**: Node.js 22 ships with npm v10, but npm OIDC requires npm v11.5.1+ (shipped with Node.js 24).

## Decision

Use Node.js 24 and support both OIDC and token-based authentication:

1. **Upgrade to Node.js 24** - Required for npm v11+ OIDC support
2. **Add NPM_TOKEN fallback** - If token provided, use it; otherwise try OIDC

## Implementation

```yaml
# Before (broken)
- name: Setup Node.js
  uses: actions/setup-node@v6
  with:
    node-version: '22'  # ❌ npm v10 - OIDC fails

# After (fixed)
- name: Setup Node.js
  uses: actions/setup-node@v6
  with:
    node-version: '24'  # ✅ npm v11+ required for OIDC
```

## Alternative Solutions Considered

| Option | Status | Notes |
|--------|--------|-------|
| OIDC only | ❌ Failed | Requires package to exist first, Node 24 needed |
| npm token | ✅ Works | Classic approach, requires secret |
| Node 24 + token fallback | ✅ Chosen | Works now, OIDC can be enabled later |

## Consequences

- **Positive**: Fixes immediate publishing issue
- **Negative**: Requires NPM_TOKEN secret for reliable publishing
- **Neutral**: OIDC still requires manual Trusted Publisher setup in npm UI

## References

- [NPM Trusted Publishing: The "Weird" 404 Error and Node.js 24 Fix](https://medium.com/@kenricktan11/npm-trusted-publishers-the-weird-404-error-and-the-node-js-24-fix-a9f1d717a5dd)
- npm CLI v11.5.1+ required for OIDC
