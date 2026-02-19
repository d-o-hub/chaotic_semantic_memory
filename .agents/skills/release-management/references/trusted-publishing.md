# Trusted Publishing

OIDC-based authentication for crates.io and npm without long-lived tokens.

## Overview

Trusted Publishing uses OpenID Connect (OIDC) to verify CI/CD workflow identity and issue short-lived tokens (30 min expiry). This eliminates the security risks of long-lived API tokens.

```
┌──────────────┐     ┌─────────────┐     ┌───────────────┐
│ GitHub       │────▶│ OIDC Token  │────▶│ crates.io/npm │
│ Actions      │     │ (short-lived)│     │ (verifies)    │
└──────────────┘     └─────────────┘     └───────────────┘
```

## crates.io Trusted Publishing

### Requirements

- Rust stable toolchain
- Crate must exist on crates.io (initial publish requires manual token)
- Repository on GitHub or GitLab

### Setup Steps

1. **Initial Publish** (one-time, requires API token):
   ```bash
   # Login to crates.io
   cargo login
   
   # Publish initial version
   cargo publish
   ```

2. **Configure Trusted Publishing**:
   - Go to `https://crates.io/crates/CRATE_NAME/settings`
   - Navigate to "Trusted Publishing"
   - Click "Add"
   - Select platform: GitHub
   - Enter:
     - Repository: `d-o-hub/chaotic_semantic_memory`
     - Workflow: `.github/workflows/release.yml`
     - Environment: (optional)

3. **Enable "Trusted Publishing Only" Mode** (recommended):
   - In crate settings, enable this to block token-based publishing
   - Only OIDC-authenticated workflows can publish

### GitHub Actions Configuration

```yaml
jobs:
  publish:
    runs-on: ubuntu-latest
    permissions:
      id-token: write  # Required for OIDC token
      contents: read
    
    steps:
      - uses: actions/checkout@v4
      
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Publish to crates.io
        run: cargo publish
        # No CARGO_REGISTRY_TOKEN needed!
```

### Verification

```bash
# Check crate publishing settings
curl -s https://crates.io/api/v1/crates/chaotic_semantic_memory | jq '.versions[0]'

# Verify OIDC token in workflow (debug)
- name: Debug OIDC
  run: echo $ACTIONS_ID_TOKEN_REQUEST_TOKEN | head -c 20
```

## npm Trusted Publishing with Provenance

### Requirements

- Node.js 24+
- npm 11+
- Package must exist on npm (initial publish is manual)
- GitHub repository with Actions

### Setup Steps

1. **Initial Publish** (manual, requires login):
   ```bash
   npm login
   npm publish
   ```

2. **Configure Trusted Publishing**:
   - Go to `https://www.npmjs.com/package/PACKAGE_NAME/access`
   - Enable "Trusted Publishing"
   - Configure GitHub repository

3. **Update package.json**:
   ```json
   {
     "name": "chaotic_semantic_memory",
     "version": "1.0.0",
     "repository": {
       "type": "git",
       "url": "git+https://github.com/d-o-hub/chaotic_semantic_memory.git"
     }
   }
   ```

### GitHub Actions Configuration

```yaml
jobs:
  publish-npm:
    runs-on: ubuntu-latest
    permissions:
      id-token: write  # Required for OIDC
      contents: read
    
    steps:
      - uses: actions/checkout@v4
      
      - uses: actions/setup-node@v4
        with:
          node-version: '24'
          registry-url: 'https://registry.npmjs.org'
          
      - name: Install latest npm
        run: npm install -g npm@latest
        
      - name: Publish with provenance
        run: npm publish --provenance
        # No NODE_AUTH_TOKEN needed!
```

### Provenance Verification

Users can verify package provenance:

```bash
# View provenance info
npm view chaotic_semantic_memory time

# Verify with sigstore
npm audit signatures
```

## Security Benefits

| Feature | Long-lived Token | Trusted Publishing |
|---------|------------------|-------------------|
| Token lifetime | Unlimited / 90 days | 30 minutes |
| Rotation needed | Yes | No |
| Leak risk | High | Minimal |
| Repository verification | No | Yes |
| Workflow verification | No | Yes |
| Audit trail | Basic | Full OIDC claims |

## Multi-Platform Support

### crates.io Supported Platforms

| Platform | Status | Notes |
|----------|--------|-------|
| GitHub Actions | Stable | Primary platform |
| GitLab CI/CD | Beta | GitLab.com only |
| Codeberg/Forgejo | Planned | Contributions welcome |

### npm Supported Platforms

| Platform | Status |
|----------|--------|
| GitHub Actions | Stable |
| GitLab CI/CD | Planned |

## Troubleshooting

### "OIDC token exchange failed"

1. Verify Trusted Publishing is enabled on crates.io/npm
2. Check repository name matches exactly
3. Ensure `id-token: write` permission is set
4. Verify workflow file path matches configuration

### "crate already exists"

The version was already published. Bump version in Cargo.toml.

### "insufficient permissions"

Add to workflow:
```yaml
permissions:
  id-token: write
  contents: write
```

### "npm provenance failed"

1. Ensure Node.js 24+: `node --version`
2. Ensure npm 11+: `npm --version`
3. Verify repository field in package.json
4. Run: `npm install -g npm@latest`

### Debug OIDC Token

```yaml
- name: Debug OIDC Claims
  run: |
    curl -H "Authorization: bearer $ACTIONS_ID_TOKEN_REQUEST_TOKEN" \
      "$ACTIONS_ID_TOKEN_REQUEST_URL&audience=crates.io"
```

## Environment-based Publishing

For additional security, use GitHub Environments:

```yaml
jobs:
  publish:
    runs-on: ubuntu-latest
    environment: production  # Requires approval
    permissions:
      id-token: write
```

Configure environment protection rules:
- Required reviewers
- Wait timer
- Deployment branches (main only)
