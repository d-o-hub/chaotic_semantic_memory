# v{version} Tag Format Best Practices

GitHub release management follows the `v{version}` convention for all tags and releases.

## Why v{version}?

- **Standard**: Industry convention used by major projects (Go, Node, Rust, etc.)
- **Clarity**: `v` prefix distinguishes version tags from branch names
- **Sorting**: GitHub releases sort correctly with v-prefixed versions
- **Security**: Prevents namespace conflicts with branch names

## Tag Format Specification

```
vMAJOR.MINOR.PATCH[-PRERELEASE]
```

| Example | Type |
|---------|------|
| `v1.0.0` | Release |
| `v1.2.0` | Minor release |
| `v2.0.0-beta.1` | Pre-release |
| `v1.0.1` | Patch release |

## Rolling Tags (2026 Best Practice)

Automatically update major and minor tags after each release:

| Tag | Points To | Example |
|-----|-----------|---------|
| `v1` | Latest v1.x.x | v1.2.0 |
| `v1.2` | Latest v1.2.x | v1.2.3 |
| `v2` | Latest v2.x.x | v2.0.0 |

### Workflow Addition

```yaml
  update-rolling-tags:
    runs-on: ubuntu-latest
    needs: create-github-release
    steps:
      - uses: actions/checkout@v6

      - name: Update rolling tags
        uses: cssnr/update-version-tags-action@v2
        with:
          prefix: v
          major: true
          minor: true
```

## Tag Naming Rules

1. **Always** use `v` prefix for tags
2. **Never** use `v` prefix in Cargo.toml version field
3. **Always** match tag version to Cargo.toml version
4. **Never** reuse existing tags

## Git Commands

```bash
# Create annotated tag
git tag -a v1.2.0 -m "Release 1.2.0"

# Push tag
git push origin v1.2.0

# Delete local tag
git tag -d v1.2.0

# Delete remote tag
git push --delete origin v1.2.0

# List tags
git tag -l "v*"
```

## CI Tag Validation

The release workflow validates tag format:

```bash
if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    fail "Invalid tag format. Expected: v1.2.3"
fi
```

## Security Considerations

- Require signed commits for release tags (GPG)
- Use GitHub's tag protection rules
- Enable branch protection on main
- Use OIDC tokens for publishing (never long-lived tokens)

## References

- [Semantic Versioning](https://semver.org)
- [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [update-version-tags-action](https://github.com/marketplace/actions/update-version-tags-action)
