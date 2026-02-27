# Manual TODO - Items Requiring Human Action

This file tracks pending manual actions that cannot be automated and require human intervention.

## npm Publishing (ADR-0046)

### 1. Manual First npm Publish

**Status**: ✅ Completed (2026-02-27)
**ADR**: ADR-0046 - npm OIDC Trusted Publishing

**Completed Actions**:
- Built WASM package with `wasm-pack build --target web --scope d-o-hub`
- Published v0.1.0 to npm with provenance
- Package now available at: https://www.npmjs.com/package/@d-o-hub/chaotic_semantic_memory

**Note**: npm auto-corrected `repository.url` to `git+https://` format. Run `npm pkg fix` in `pkg/` directory to normalize this for future builds.

---

### 2. Configure npm Trusted Publisher

**Status**: 🔄 Ready to Complete (v0.1.0 published, can now configure)
**ADR**: ADR-0046 - npm OIDC Trusted Publishing

**Action Required**:
1. Go to https://www.npmjs.com/package/@d-o-hub/chaotic-semantic_memory/access
2. Under "Trusted Publisher", click "Add GitHub Actions"
3. Configure:
   - Organization: `d-o-hub`
   - Repository: `chaotic_semantic_memory`
   - Workflow: `npm-publish.yml`
4. Click "Set up connection"

**Verification**:
- Trusted Publisher shows "GitHub Actions" as connected

---

### 3. Verify CI Publishing Works

**Status**: Pending
**ADR**: ADR-0046 - npm OIDC Trusted Publishing

**Action Required**:
1. Create a new version tag:
   ```bash
   git tag v0.1.1
   git push origin v0.1.1
   ```

2. Monitor GitHub Actions workflow at:
   https://github.com/d-o-hub/chaotic_semantic_memory/actions

3. Verify:
   - Workflow shows "Signed provenance statement" in logs
   - Package version updates on npmjs.com
   - No manual npm token required in workflow

---

## Documentation Updates

### 4. Update CHANGELOG.md

**Status**: Pending

**Action Required**:
- Add entry for v0.1.0 release with features, bug fixes, and breaking changes
- Update version number in Cargo.toml if releasing

---

### 5. Review Security Vulnerability

**Status**: Pending

**Action Required**:
- Visit: https://github.com/d-o-hub/chaotic_semantic_memory/security/dependabot
- Review and address any reported vulnerabilities
- Update dependencies if needed

---

## Completed Items

| Item | Completed | Date |
|------|-----------|------|
| **WASM-pack bulk memory fix (ADR-0048)** | ✅ | 2026-02-27 |
| All Phase 27 (Wave 13) actions | ✅ | 2026-02-26 |
| All Phase 26B actions (except manual) | ✅ | 2026-02-20 |
| All Phase 25 (Wave 11) actions | ✅ | 2026-02-19 |
| All Phase 24 (Wave 10) actions | ✅ | 2026-02-18 |
| All Wave 9 (CLI) actions | ✅ | 2026-02-17 |
| All Wave 8 actions | ✅ | 2026-02-16 |
| All Wave 7 actions | ✅ | 2026-02-15 |
| All Wave 6 actions | ✅ | 2026-02-14 |
| All Wave 5 actions | ✅ | 2026-02-13 |
