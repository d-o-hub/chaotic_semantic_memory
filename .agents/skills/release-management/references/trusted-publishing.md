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

### Which events can mint a crates.io token

`crates_io_api::handlers::github::handle_github_token_inner` **refuses** OIDC assertions
for `workflow_run` and `pull_request_target`. A publishing job that lives inside a
`workflow_run`-triggered workflow (which is how this repo's `release.yml` fires) therefore
cannot be fixed in place — the trigger has to move. That is why crates.io publishing lives
in `publish-crates.yml` on `workflow_dispatch`, and why a tag trigger is not the answer
either: `release.yml` pushes the tag with the runner's own `GITHUB_TOKEN`
(`release.yml:237-242`), and a `GITHUB_TOKEN` push spawns no workflow run at all.

What the event rules do **not** forbid is the *dispatch*. GitHub's recursion guard is
documented to exempt `workflow_dispatch` and `repository_dispatch`, so a job inside the
release run may POST a dispatch and get a real run out of it — which is exactly what
`release.yml`'s `trigger-crates-io-publish` does, then waits for that run's verdict. The
publishing token is still minted only in the dispatched run, on an accepted event.

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

2. **Configure Trusted Publishing** — once **per crate**, in the crates.io web UI only
   (there is no API to create or list these configs, so nothing in CI can pre-flight them):
   - Go to `https://crates.io/crates/CRATE_NAME/management` and open "Trusted Publishing"
   - Add a trusted publisher with:
     - Owner: `d-o-hub`
     - Repository: `chaotic_semantic_memory`
     - Workflow filename: `publish-crates.yml` (the filename, not a path)
     - Environment: `crates.io` — must match the job's `environment:` exactly
   - This repo has **nine** crates (`csm-chaos`, `csm-core-lib`, `csm-traits`,
     `csm-embedding`, `csm-memory`, `csm-retrieval`, `csm-persistence`,
     `chaotic_semantic_memory`, `csm-duckdb`); a config gap on one of them 403s that one
     crate and lets the other eight publish, because the minted token is scoped to the
     subset whose configs match. The last step of `publish-crates.yml` re-probes the sparse
     index for all nine and reds the run on any gap.

3. **Enable "Trusted Publishing Only" Mode** — only after all nine configs exist and the
   recovery dispatch has gone green. Enabling it earlier blocks the token path you may still
   need to finish the set.

### GitHub Actions Configuration

`cargo` still reads `CARGO_REGISTRY_TOKEN` from the environment; what changes is where the
value comes from — the OIDC exchange's output, not a repository secret.

```yaml
jobs:
  publish:
    runs-on: ubuntu-latest
    environment: crates.io
    permissions:
      contents: read
      id-token: write  # Required for OIDC token
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18 # v1.0.5
        id: crates-auth
      - name: Publish to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ steps.crates-auth.outputs.token }}
        run: cargo publish
```

The minted token lives 30 minutes (`ACCESS_TOKEN_LIFETIME`), so keep the publishing job's
`timeout-minutes` under that — a run that outlives its credential fails as a 403 and reads
like a config problem. The job that *dispatches* this one needs no `id-token: write` at all:
holding no token is the point, and its only crates.io job is to POST the dispatch, match the
run it caused by that run's `head_sha`, and fail if the run fails or never finishes.

### Verification

```bash
# Check the index row for the version the dispatch claimed (shard from scripts/crates-io-index-path.sh)
curl -sL "https://index.crates.io/$(bash scripts/crates-io-index-path.sh chaotic_semantic_memory)" \
  | grep -cF '"vers":"0.3.9"'
```

`/api/v1/me` is not a usable probe: crates.io serves it behind `AuthCheck::only_cookie()`,
so a bearer token — valid or not — never returns 200. Do not build a gate on it.

## npm Trusted Publishing with Provenance (2026)

### Requirements

- Node.js 24+ (verified - workflow uses ubuntu-24.04)
- npm 11.5.1+
- Package must exist on npm (initial publish requires token)
- GitHub repository with Actions
- 2FA enabled on npm account

### Setup Steps (Official 2026 Process)

1. **Initial Publish** (one-time, requires npm automation token):
   ```bash
   npm login --registry=https://registry.npmjs.org/
   # Use automation token from https://www.npmjs.com/settings/tokens
   cd pkg
   npm publish --access public
   ```

2. **Configure Trusted Publisher on npmjs.com**:
   - Go to: `https://www.npmjs.com/package/@d-o-hub/chaotic_semantic_memory/settings`
   - Find "Trusted Publishing" section
   - Click "Select your publisher" → GitHub Actions
   - Configure:
     - **Organization**: `d-o-hub`
     - **Repository**: `chaotic_semantic_memory`
     - **Workflow filename**: `release.yml` (just filename, not path!) — measured, not
       copied: this repo has no `npm-publish.yml`, and 0.3.9 was published with provenance
       by `release.yml`'s `publish-npm-wasm` (environment `npm`) and `publish-npm-cli`
       (environment `npm-cli`) jobs.
     - **Environment**: `npm` for the WASM package, `npm-cli` for the CLI package — each must
       match the publishing job's `environment:`

3. **Update GitHub Actions workflow** (already done):
   - Ensure `id-token: write` permission
   - Use Node 24+ (ubuntu-24.04 runner)
   - Run `npm install -g npm@latest`

### GitHub Actions Configuration

```yaml
permissions:
  contents: read
  id-token: write  # Required for OIDC

jobs:
  publish-npm:
    runs-on: ubuntu-24.04
    environment: npm  # Optional: matches trusted publisher config
    steps:
      - uses: actions/checkout@v6

      - uses: actions/setup-node@v6
        with:
          node-version: '24'
          registry-url: 'https://registry.npmjs.org'

      - name: Install latest npm
        run: npm install -g npm@latest

      - name: Publish with provenance
        run: npm publish --provenance --access public
```

### Common Issues (2026)

| Error | OIDC Status | Root Cause | Solution |
|-------|-------------|------------|----------|
| "404 Not Found" | Signed ✓ | Package doesn't exist | Do initial publish with NPM_TOKEN first |
| "404 Not Found" | Signed ✓ | **Workflow filename mismatch** | Update npmjs Trusted Publisher workflow name |
| "404 Not Found" | Signed ✓ | Environment mismatch | Remove environment or match exactly |
| "Access token expired" | N/A | Token-based publish | Token revoked; generate new |
| "OIDC token exchange failed" | Not signed | Trusted Publisher missing | Configure on npmjs.com |
| "Workflow filename" mismatch | N/A | Wrong workflow configured | Use just `filename.yml`, no path |

### Critical: Workflow Filename Matching (2026-04)

**The #1 cause of E404 OIDC failures is workflow filename mismatch.**

npm Trusted Publishers verify the OIDC token's `workflow` claim against the configured workflow filename:

```
npmjs config: release.yml → OIDC workflow claim: release.yml → ✓ MATCH
npmjs config: release.yml → OIDC workflow claim: npm-publish.yml → ✗ E404
```

**Real-world failure pattern**:
- v0.3.1: Published from `release.yml` → OIDC success
- v0.3.2: Published from `npm-publish.yml` → OIDC E404 (same package, same Trusted Publisher)

**Solution**: Configure Trusted Publisher for ALL workflows that publish:
1. Go to npmjs.com → Package → Settings → Trusted Publishers
2. Add entries for each workflow:
   - `release.yml` (for unified releases)
   - `npm-publish.yml` (for tag-triggered standalone)

### Verification

After successful publish:
```bash
# Check package exists
npm view @d-o-hub/chaotic_semantic_memory

# Verify provenance (shows sigstore transparency log)
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

`id-token: write` is what the OIDC exchange needs; `contents: write` does not help and a
publishing job has no business holding it. Keep the pair `contents: read` +
`id-token: write` under a repository-wide `permissions: {}`.

### "npm provenance failed"

1. Ensure Node.js 24+: `node --version`
2. Ensure npm 11+: `npm --version`
3. Verify repository field in package.json
4. Run: `npm install -g npm@latest`

## Environment-based Publishing

Binding the publisher config to a GitHub environment is what makes the assertion
specific: the crates.io config must carry **exactly** the environment the job declares, or
the exchange fails for that crate. This repo uses `environment: crates.io`.

Keep that environment's `protection_rules` empty. A wait timer or required reviewers hold
the job *wherever it sits in the run*, and a publish job that pauses after four of nine
crates uploaded leaves a partial publish — the state this doc's whole reconciliation step
exists to escape. Measure rather than assume:
`gh api repos/{owner}/{repo}/environments --jq '.environments[] | "\(.name) \(.protection_rules)"'`.
