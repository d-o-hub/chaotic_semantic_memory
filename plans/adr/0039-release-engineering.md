# ADR-0039: Release Engineering Strategy

## Status

Proposed

## Context and Problem Statement

The `chaotic_semantic_memory` crate is approaching its 1.0 release and needs a comprehensive release engineering strategy. Manual release processes are error-prone, time-consuming, and can lead to:

**Current Challenges:**
- Version number management requires manual updates across multiple files
- Changelog generation is tedious and often incomplete
- Publishing to crates.io requires manual token management and API keys
- Documentation deployment requires separate workflow configuration
- Inconsistent release notes between GitHub releases and crates.io

**Security Concerns:**
- Long-lived API tokens for crates.io create security risk
- Token rotation requires coordinated updates across repositories
- No cryptographic proof of release provenance

**2026 Best Practices Research:**
- **semantic-release**: Automates versioning and changelog generation based on conventional commits
- **crates.io Trusted Publishing**: Uses OIDC (OpenID Connect) for passwordless authentication—no API tokens needed
- **npm Trusted Publishing**: Requires `--provenance` flag and `permissions: id-token: write` in GitHub Actions
- **GitHub Pages + mdBook**: Industry-standard for Rust project documentation hosting

## Decision Drivers

1. **Automation First**: Reduce manual steps to prevent human error
2. **Security**: Eliminate long-lived secrets; use OIDC trusted publishing
3. **Provenance**: Cryptographic proof of build integrity and source
4. **Conventional Commits**: Leverage existing commit convention for versioning
5. **Developer Experience**: Single workflow for release, no manual steps
6. **Documentation**: Automated doc deployment aligned with releases

## Considered Options

### Option 1: Manual Releases (Status Quo)

Manually bump versions, update changelog, and publish via `cargo publish`.

**Pros:**
- Full control over every step
- No additional tooling required
- Simple for infrequent releases

**Cons:**
- Error-prone (forgot changelog, wrong version)
- Requires API token management
- No provenance/attestation
- Time-consuming
- Inconsistent release quality

### Option 2: semantic-release + Trusted Publishing + mdBook (Chosen)

Automated versioning via semantic-release, OIDC trusted publishing to crates.io, and mdBook for documentation.

**Pros:**
- Fully automated releases from conventional commits
- No API tokens (OIDC trusted publishing)
- Cryptographic provenance
- Consistent changelog generation
- Single workflow trigger
- Industry-standard tooling (2026)

**Cons:**
- Initial setup complexity
- Requires conventional commits discipline
- Dependency on external tools
- Learning curve for team

### Option 3: cargo-release + Standard Publishing

Use `cargo-release` for automation with traditional API token publishing.

**Pros:**
- Rust-native tooling
- Well-established in Rust ecosystem
- Simpler than semantic-release

**Cons:**
- Requires crates.io API token
- No OIDC/trusted publishing
- Limited changelog capabilities
- Separate docs deployment needed

### Option 4: Release-please + Provenance

Google's release-please with SLSA provenance.

**Pros:**
- Strong provenance story
- Multi-language support
- Well-maintained

**Cons:**
- More complex than semantic-release for single-language project
- Provenance overhead for small crate
- Less idiomatic for Rust ecosystem

## Decision Outcome

Chosen option: **Option 2 - semantic-release + Trusted Publishing + mdBook**

This provides the best balance of automation, security, and ecosystem alignment for 2026 best practices.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Release Pipeline                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [main branch]                                                   │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────┐                                             │
│  │ Conventional    │  feat: → minor, fix: → patch,               │
│  │ Commits         │  BREAKING CHANGE: → major                   │
│  └────────┬────────┘                                             │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────┐     ┌─────────────────┐                     │
│  │ semantic-release│────▶│ Version Bump    │                     │
│  │ Analysis        │     │ CHANGELOG.md    │                     │
│  └────────┬────────┘     └─────────────────┘                     │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────┐                                             │
│  │ crates.io       │  OIDC Trusted Publishing                    │
│  │ (no API token)  │  permissions: id-token: write               │
│  └────────┬────────┘                                             │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────┐                                             │
│  │ GitHub Release  │  Auto-generated notes                       │
│  │ + Provenance    │  Cryptographic attestation                  │
│  └────────┬────────┘                                             │
│           │                                                      │
│           ▼                                                      │
│  ┌─────────────────┐                                             │
│  │ mdBook Docs     │  docs.anomaly.co/chaotic_semantic_memory    │
│  │ GitHub Pages    │  Updated on every release                   │
│  └─────────────────┘                                             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Details

#### 1. semantic-release Configuration

`.releaserc.json`:
```json
{
  "branches": ["main"],
  "plugins": [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    "@semantic-release/exec", {
      "publishCmd": "cargo publish --token ${CRATES_IO_TOKEN}"
    },
    "@semantic-release/github",
    "@semantic-release/git"
  ]
}
```

#### 2. crates.io Trusted Publishing (OIDC)

GitHub Actions workflow for trusted publishing:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: write
  id-token: write  # Required for OIDC trusted publishing
  pages: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '22'
          
      - name: Install semantic-release
        run: npm install -g semantic-release @semantic-release/exec
        
      - name: Configure crates.io trusted publishing
        run: |
          # OIDC token automatically provided by GitHub Actions
          # No API token needed - crates.io trusts GitHub's OIDC
          cargo login --registry crates-io
          
      - name: Release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: semantic-release
        
  docs:
    needs: release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Install mdBook
        run: cargo install mdbook
        
      - name: Build Documentation
        run: |
          mdbook build docs/
          cargo doc --no-deps --target-dir docs/html/api
          
      - name: Deploy to GitHub Pages
        uses: actions/deploy-pages@v4
        with:
          artifact_name: docs
```

#### 3. mdBook Structure

```
docs/
├── book.toml           # mdBook configuration
├── src/
│   ├── SUMMARY.md      # Table of contents
│   ├── introduction.md
│   ├── getting-started/
│   │   ├── installation.md
│   │   ├── quick-start.md
│   │   └── configuration.md
│   ├── architecture/
│   │   ├── overview.md
│   │   ├── hypervectors.md
│   │   └── reservoir.md
│   ├── api/
│   │   └── (auto-generated from cargo doc)
│   └── examples/
│       ├── basic-usage.md
│       └── advanced-patterns.md
└── theme/
    └── custom.css
```

`book.toml`:
```toml
[book]
title = "Chaotic Semantic Memory"
authors = ["Anomaly"]
description = "AI memory systems with hyperdimensional vectors and chaotic reservoirs"
language = "en"
multilingual = false
src = "src"

[build]
build-dir = "book"
create-missing = false

[output.html]
git-repository-url = "https://github.com/anomalyco/chaotic_semantic_memory"
edit-url-template = "https://github.com/anomalyco/chaotic_semantic_memory/edit/main/docs/{path}"
site-url = "/chaotic_semantic_memory/"

[output.html.search]
enable = true
```

#### 4. Provenance Configuration

For npm (if publishing JS bindings later) and supply chain security:

```yaml
# Add to release workflow
- name: Generate SLSA Provenance
  uses: slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v2.0.0
  with:
    base64-subjects: |
      ${{ steps.hash.outputs.hash }}
```

### Version Strategy

| Commit Type | Version Bump | Example |
|-------------|--------------|---------|
| `feat:` | Minor (0.x.0) | `0.1.0` → `0.2.0` |
| `fix:` | Patch (0.0.x) | `0.1.0` → `0.1.1` |
| `feat!:` or `BREAKING CHANGE` | Major (x.0.0) | `0.1.0` → `1.0.0` |
| `docs:`, `chore:`, `test:` | No release | - |

### Release Triggers

1. **Automatic**: Push to `main` with conventional commits
2. **Manual**: `workflow_dispatch` for emergency releases
3. **Scheduled**: Weekly patch releases for dependency updates (optional)

### Positive Consequences

1. **Zero-Touch Releases**: Fully automated from commit to publish
2. **No Secrets Management**: OIDC eliminates API token lifecycle
3. **Cryptographic Provenance**: Verifiable build attestation
4. **Consistent Quality**: Every release follows same process
5. **Accurate Changelogs**: Generated from commit history
6. **Fast Recovery**: Easy to identify and revert problematic releases
7. **Documentation Sync**: Docs deploy with every release
8. **Security Posture**: Minimal attack surface (no stored credentials)

### Negative Consequences

1. **Conventional Commit Dependency**: Team must follow commit conventions strictly
2. **Tooling Learning Curve**: semantic-release configuration can be complex
3. **Less Control**: Automated releases mean less manual intervention
4. **Node.js Dependency**: Requires Node for semantic-release (Rust-native alternatives exist)
5. **Initial Setup Time**: ~4-8 hours for full pipeline configuration
6. **Debugging Complexity**: Automated pipelines harder to debug than manual

## Pros and Cons of Options

### Option 1: Manual Releases
- Good, because full control and simplicity
- Bad, because error-prone and requires token management
- Bad, because no provenance

### Option 2: semantic-release + Trusted Publishing (Chosen)
- Good, because fully automated
- Good, because OIDC security (no tokens)
- Good, because provenance attestation
- Good, because industry-standard 2026 practice
- Bad, because requires Node.js
- Bad, because conventional commit discipline needed

### Option 3: cargo-release
- Good, because Rust-native
- Good, because well-established
- Bad, because requires API tokens
- Bad, because no OIDC support

### Option 4: Release-please
- Good, because provenance support
- Good, because multi-language
- Bad, because complexity overkill for single crate
- Bad, because less idiomatic for Rust

## Implementation Plan

### Phase 1: Foundation (2 hours)

1. Create `.releaserc.json` configuration
2. Set up `docs/` mdBook structure
3. Create `book.toml` with project metadata
4. Add documentation stubs for all sections

### Phase 2: GitHub Actions (3 hours)

1. Create `.github/workflows/release.yml`
2. Configure OIDC trusted publishing for crates.io
3. Set up GitHub Pages deployment
4. Add SLSA provenance generation
5. Test workflow with `workflow_dispatch`

### Phase 3: crates.io Setup (1 hour)

1. Register crate name on crates.io
2. Enable trusted publishing in crates.io settings
3. Link GitHub repository to crates.io
4. Verify OIDC trust relationship

### Phase 4: Documentation (2 hours)

1. Write getting-started guide
2. Document API usage patterns
3. Add architecture overview
4. Create example gallery
5. Set up auto-deploy for `cargo doc` output

### Phase 5: Validation (1 hour)

1. Perform dry-run release
2. Verify version bump logic
3. Check changelog generation
4. Confirm docs deployment
5. Test provenance attestation

## Related ADRs

- **ADR-0038**: Cargo.toml Modernization (prerequisite for publishing)
- **ADR-0036**: CI/DX Hardening (conventional commits foundation)
- **ADR-0037**: Rust Best Practices (edition 2024 alignment)

## References

- [crates.io Trusted Publishing](https://blog.rust-lang.org/2024/10/29/crates-io-trusted-publishing.html)
- [semantic-release Documentation](https://semantic-release.gitbook.io/)
- [mdBook Documentation](https://rust-lang.github.io/mdBook/)
- [GitHub Actions OIDC](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
- [SLSA Provenance](https://slsa.dev/provenance/v0.2)
- [Conventional Commits](https://www.conventionalcommits.org/)

## Notes

**Analysis performed by:** Release Engineering Review  
**Risk Level:** Low (well-established tooling, reversible configuration)  
**Priority:** High (blocks 1.0 release)

---

**Created:** 2026-02-19  
**Author:** Architecture Team  
**Status:** Proposed (pending implementation)
