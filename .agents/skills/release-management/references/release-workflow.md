# Release Workflow

Complete release automation workflow with GitHub Actions integration.

## CI/CD Pipeline Overview

```
main branch ──▶ semantic-release ──▶ publish crates.io ──▶ deploy docs
                      │
                      ├── Analyze commits
                      ├── Calculate version
                      ├── Generate CHANGELOG
                      └── Create git tag
```

## GitHub Actions Workflow

### Release Workflow (`.github/workflows/release.yml`)

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write
  id-token: write  # Required for Trusted Publishing
  pages: write

jobs:
  publish-crates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Publish to crates.io
        run: cargo publish
        # Uses Trusted Publishing - no token needed!

  publish-npm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '24'
          registry-url: 'https://registry.npmjs.org'
          
      - name: Install latest npm
        run: npm install -g npm@latest
        
      - name: Publish with provenance
        run: npm publish --provenance
        # Uses Trusted Publishing via OIDC

  github-release:
    needs: [publish-crates]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Generate release notes
        id: notes
        run: |
          VERSION=${GITHUB_REF#refs/tags/}
          echo "notes<<EOF" >> $GITHUB_OUTPUT
          git log --pretty=format:"- %s" $(git describe --tags --abbrev=0 HEAD~1)..HEAD >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT
          
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          body: ${{ steps.notes.outputs.notes }}
          generate_release_notes: true

  deploy-docs:
    needs: [github-release]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup mdBook
        uses: peaceiris/actions-mdbook@v2
        with:
          mdbook-version: '0.4.40'
          
      - name: Build docs
        run: mdbook build docs/
        
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/book
```

### Semantic Release Workflow (`.github/workflows/semantic-release.yml`)

```yaml
name: Semantic Release

on:
  push:
    branches: [main]

permissions:
  contents: write
  issues: write
  pull-requests: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for commit analysis
          
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '24'
          
      - name: Install semantic-release
        run: |
          npm install -g semantic-release \
            @semantic-release/git \
            @semantic-release/changelog \
            @semantic-release/commit-analyzer \
            @semantic-release/release-notes-generator \
            @semantic-release/github
            
      - name: Run semantic-release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: semantic-release
```

## semantic-release Configuration

### `.releaserc.json`

```json
{
  "branches": ["main"],
  "plugins": [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    [
      "@semantic-release/git",
      {
        "assets": ["CHANGELOG.md", "Cargo.toml", "package.json"],
        "message": "chore(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}"
      }
    ],
    "@semantic-release/github"
  ]
}
```

## Commit Types → Version Mapping

| Commit Pattern | Version Bump |
|----------------|--------------|
| `feat(scope)!: description` | MAJOR (breaking) |
| `feat(scope): description` | MINOR |
| `fix(scope): description` | PATCH |
| `perf(scope): description` | PATCH |
| `docs(scope): description` | PATCH |
| `chore(scope): description` | No release |
| `test(scope): description` | No release |
| `ci(scope): description` | No release |

## Release Branches

| Branch | Purpose |
|--------|---------|
| `main` | Production releases |
| `beta` | Pre-release testing |
| `alpha` | Development releases |

## Environment Configuration

### crates.io Trusted Publishing Setup

1. Navigate to `https://crates.io/crates/chaotic_semantic_memory/settings`
2. Click "Trusted Publishing" → "Add"
3. Configure:
   - Repository: `d-o-hub/chaotic_semantic_memory`
   - Workflow: `.github/workflows/release.yml`
   - Environment: (optional, for additional controls)

### npm Trusted Publishing Setup

1. First publish must be manual: `npm publish`
2. Navigate to `https://www.npmjs.com/package/@d-o-hub/chaotic_semantic_memory/access`
   (and `https://www.npmjs.com/package/@d-o-hub/csm/access` for the CLI)
3. Enable "Trusted Publishing"
4. Configure GitHub repository

## Pre-Release Checklist

- [ ] All PRs merged to main
- [ ] CI passing on main branch
- [ ] CHANGELOG.md updated (or will be by semantic-release)
- [ ] Version bump in Cargo.toml matches tag
- [ ] Documentation updated
- [ ] Breaking changes documented
- [ ] Trusted Publishing verified

## ⚠️ Common Pitfalls

### Release Name Format
Always use `v{version}` format for release names, NOT `{package} v{version}`:

```yaml
# ✅ Correct
name: v${{ needs.validate.outputs.version }}

# ❌ Wrong - hardcoded package name
name: chaotic_semantic_memory v${{ needs.validate.outputs.version }}
```

### body_path vs generate_release_notes
These options are **mutually exclusive** in `softprops/action-gh-release`:

```yaml
# ✅ Correct - use body_path to include custom changelog
body_path: release_notes.md
generate_release_notes: false  # or omit entirely

# ❌ Wrong - generate_release_notes overrides body_path
body_path: release_notes.md
generate_release_notes: true  # This will IGNORE your body_path!
```

When `generate_release_notes: true`, the action auto-generates minimal release notes (just version + compare link) and ignores any `body` or `body_path` you provide.

## Post-Release Checklist

- [ ] crates.io page shows new version
- [ ] GitHub Release created with notes
- [ ] Docs deployed to GitHub Pages
- [ ] Announcement posted (if major/minor)
