# PR #77 Validation Report (d-o-hub/chaotic_semantic_memory)

Date: 2026-04-14
Reviewer: Codex

## Scope
Validated the changes introduced by PR #77:
- `cli-npm/postinstall.js`
- `scripts/sync-version.sh`

## Findings

### 1) `cli-npm/postinstall.js`
- **Change reviewed**: Removed `createGunzip()` from stream pipeline before passing data to `tar -xz`.
- **Assessment**: ✅ Correct fix.
- **Why**: `tar -xz` already performs gzip decompression. Pre-decompressing with `createGunzip()` causes a double-decompression path and extraction failures.

### 2) `scripts/sync-version.sh`
- **Change reviewed**: Updated duplicate-version detection to use:
  - `grep -c "^## \[${ver}\]" "$changelog" || true`
  - `if [ "${existing_count:-0}" -gt 0 ]; then`
- **Assessment**: ✅ Correct hardening.
- **Why**: Avoids brittle behavior under strict shell settings and handles empty/unset counts safely.

## Validation Executed

### Repro of gzip extraction behavior
A local repro showed:
- `gunzip | tar -xz` → exit code 2 (fails)
- `tar -xz` on `.tar.gz` stream → exit code 0 (succeeds)

This supports the `postinstall.js` change.

### npm install path test
Attempted package install from packed tarball (`npm pack` + `npm install --foreground-scripts`).
- Result: install failed during postinstall because the release artifact URL returned 404 for `v0.3.2/csm-linux-x64.tar.gz`.
- Interpretation: failure is due to missing release artifact, not the stream pipeline change.

## Conclusion
- PR #77 changes are sound for the code paths modified.
- CLI install success still depends on GitHub release assets existing for the package version/platform.
- Recommended follow-up: add a CI/Release check that validates all expected `csm-<platform>.tar.gz` assets exist before publishing `@d-o-hub/csm`.
