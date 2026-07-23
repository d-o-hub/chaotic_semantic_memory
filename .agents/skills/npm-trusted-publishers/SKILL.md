---
name: npm-trusted-publishers
description: Configure and troubleshoot npm OIDC Trusted Publishing for GitHub Actions. Use when npm publish fails with E404, OIDC errors, or provenance issues.
---

# npm Trusted Publishers Skill

Comprehensive guide for npm OIDC Trusted Publishing with pre-flight validation, error handling, and troubleshooting.

## Root Cause Analysis Pattern

When npm publish fails with **E404 Not Found** after OIDC provenance signing:

```
npm notice publish Signed provenance statement with source and build information from GitHub Actions
npm notice publish Provenance statement published to transparency log: https://search.sigstore.dev/?...
npm error code E404
npm error 404 Not Found - PUT https://registry.npmjs.org/@scope/package - Not found
```

**This is NOT a missing package error** - the OIDC token was generated and provenance was signed. The E404 means npm's Trusted Publisher configuration doesn't match the workflow claims.

### Workflow Filename Mismatch (Most Common)

npm Trusted Publishers verify the OIDC token's workflow claim against the configured workflow filename.

| Scenario | Result |
|----------|--------|
| npmjs config: `release.yml` | ✓ OIDC matches |
| Actual workflow: `release.yml` | ✓ Publish succeeds |
| npmjs config: `release.yml` | ✗ OIDC mismatch |
| Actual workflow: `npm-publish.yml` | ✗ E404 error |

**Solution**: Either:
1. Configure Trusted Publisher for `npm-publish.yml` (if that's your workflow)
2. Publish from `release.yml` only (if Trusted Publisher configured for it)
3. Configure both workflow filenames (comma-separated or multiple entries)

## Pre-Flight Validation Checklist

Before any release, verify Trusted Publisher configuration:

### 1. Verify npmjs.com Configuration

```
Go to: https://www.npmjs.com/package/@SCOPE/PACKAGE/settings/trusted-publishers

Check:
- Repository: OWNER/REPO (exact match)
- Workflow filename: WORKFLOW.yml (just filename, no path)
- Environment: optional (must match if set)
```

### 2. Verify GitHub Actions Workflow

```yaml
# Required permissions (MUST be at job or workflow level)
permissions:
  id-token: write  # Required for OIDC token generation
  contents: read   # Required for checkout

# Required environment (if configured in Trusted Publisher)
jobs:
  publish:
    environment: npm  # Must match npmjs.com config if set
```

### 3. Verify Workflow Filename Match

```bash
# Get the workflow that will actually publish
WORKFLOW_NAME=$(basename .github/workflows/release.yml)
echo "Workflow: $WORKFLOW_NAME"

# Verify against npmjs.com Trusted Publishers settings
# Must match EXACTLY (case-sensitive)
```

### 4. OIDC Token Debug Step

Add this step before `npm publish` to verify OIDC token claims:

```yaml
- name: Debug OIDC token claims
  run: |
    echo "Requesting OIDC token..."
    OIDC_TOKEN=$(curl -s -H "Authorization: bearer $ACTIONS_ID_TOKEN_REQUEST_TOKEN" \
      "${ACTIONS_ID_TOKEN_REQUEST_URL}&audience=npm" | jq -r '.value')

    if [ -z "$OIDC_TOKEN" ] || [ "$OIDC_TOKEN" = "null" ]; then
      echo "::error::OIDC token generation failed"
      echo "Check: id-token: write permission is set"
      exit 1
    fi

    echo "OIDC token received (first 50 chars): ${OIDC_TOKEN:0:50}..."

    # Decode and show claims (for debugging)
    echo "$OIDC_TOKEN" | cut -d'.' -f2 | base64 -d 2>/dev/null | jq . 2>/dev/null || \
      echo "Could not decode token claims"
```

## Common Error Matrix

See [error-matrix.md](references/error-matrix.md) for the full troubleshooting table.

## Configuration Steps

### Initial Setup (First-time only)

1. **Create package on npm** (requires one-time token):
   ```bash
   # Generate automation token at npmjs.com/settings/tokens
   npm login --registry=https://registry.npmjs.org/

   # First publish (requires token)
   npm publish --access public
   ```

2. **Configure Trusted Publisher**:
   - Go to: `https://www.npmjs.com/package/@SCOPE/PACKAGE/settings`
   - Scroll to "Trusted Publishers"
   - Click "Add publisher" → "GitHub Actions"
   - Configure:
     - **Repository owner**: `OWNER` (e.g., `d-o-hub`)
     - **Repository name**: `REPO` (e.g., `chaotic_semantic_memory`)
     - **Workflow filename**: `release.yml` (just the filename!)
     - **Environment**: Leave empty OR set exactly matching workflow

3. **Verify configuration**:
   ```bash
   # Check package page shows Trusted Publisher badge
   curl -s https://registry.npmjs.org/@SCOPE/PACKAGE | jq '.versions[-1].provenance'
   ```

### Workflow Configuration

```yaml
name: Release

on:
  push:
    branches: [main]

permissions:
  contents: write
  id-token: write  # CRITICAL for OIDC

jobs:
  publish-npm:
    runs-on: ubuntu-24.04
    # environment: npm  # Only if configured in Trusted Publisher
    steps:
      - uses: actions/checkout@v6

      - uses: actions/setup-node@v6
        with:
          node-version: '24'
          registry-url: 'https://registry.npmjs.org'

      - name: Install latest npm
        run: npm install -g npm@latest

      - name: Verify OIDC token available
        run: |
          if [ -z "$ACTIONS_ID_TOKEN_REQUEST_TOKEN" ]; then
            echo "::error::OIDC not available - check id-token: write permission"
            exit 1
          fi
          echo "OIDC endpoint available"

      - name: Publish with provenance
        run: |
          # Try OIDC first (no token needed)
          if npm publish --provenance --access public 2>&1 | tee /tmp/npm-log; then
            echo "Published with OIDC Trusted Publishing"
          else
            # Check if OIDC failed due to Trusted Publisher config
            if grep -q "404" /tmp/npm-log; then
              echo "::error::OIDC 404 - Trusted Publisher not configured for this workflow"
              echo "::error::Configure at: npmjs.com/package/@SCOPE/PACKAGE/settings/trusted-publishers"
              echo "Expected workflow: $(basename $GITHUB_WORKFLOW)"
              exit 1
            fi
            # Fallback to NPM_TOKEN if OIDC failed for other reasons
            if [ -n "${NODE_AUTH_TOKEN:-}" ]; then
              echo "OIDC failed, using NPM_TOKEN fallback"
              npm publish --access public
            else
              echo "::error::OIDC failed and NPM_TOKEN not available"
              exit 1
            fi
          fi
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

## Recovery from Failed Release

### Check Current State

```bash
# Check npm published versions
npm view @SCOPE/PACKAGE versions --json

# Check crates.io versions
curl -s https://crates.io/api/v1/crates/PACKAGE/versions | jq '.versions[].num'

# Check GitHub releases
gh release list

# Check deployment status
gh api repos/OWNER/REPO/deployments --jq '.[] | select(.environment == "npm") | {id: .id, state: (.statuses_url | @base64)}'
```

### Re-publish Failed Version

If npm publish failed but other registries succeeded:

```bash
# Option 1: Manual publish with NPM_TOKEN
npm publish --access public

# Option 2: Fix Trusted Publisher config and re-run workflow
# 1. Update npmjs.com Trusted Publishers settings
# 2. Trigger workflow manually:
gh workflow run release.yml -f version=0.3.2
```

## References

- [npm Trusted Publishers Documentation](https://docs.npmjs.com/trusted-publishers)
- [npm Provenance Statements](https://docs.npmjs.com/generating-provenance-statements)
- [GitHub OIDC Requirements](https://github.blog/security/supply-chain-security/understanding-githubs-requirements-for-npm-trusted-publishing-with-oidc/)
- [npm CLI OIDC Wiki](https://github.com/npm/cli/wiki/Configuring-OIDC-for-npm-publishing)