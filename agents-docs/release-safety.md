# Release Safety Requirements

**CRITICAL: Never release with failing CI. The release workflow has a guardrail that waits for CI to pass.**

## Artifact Channels
- **Rust Library:** `chaotic_semantic_memory` (crates.io / cargo)
- **JS/WASM Library:** `@d-o-hub/chaotic_semantic_memory` (npm WASM)
- **CLI Tool:** `@d-o-hub/csm` (npm CLI)
Refer to `.agents/skills/dist-channel-selection/SKILL.md` for canonical commands.

## Pre-Release Checklist (MANDATORY)
1. **Verify CI passes on all platforms**: `gh run list --workflow=ci.yml --limit 3`
2. **Ensure Cargo.lock is synchronized**: `cargo build --release && git add Cargo.lock`
3. **Check existing releases**: `gh release list --limit 5`
4. **Validate changelog entry exists**: `grep -q "^## \[${VERSION}\]" CHANGELOG.md`
5. **Verify crates.io availability (first publish)**: check workspace names via `cargo search`
6. **Check crates.io wait timer**: `gh api repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments`

## Version Bump Workflow
1. Update `Cargo.toml` version
2. Update `wasm/package.json` version
3. Update `CHANGELOG.md` with new section
4. Run `cargo build --release` to sync Cargo.lock
5. Commit all version files together (atomic)
6. Push and wait for CI to pass; only then create tag/release

## Platform-Specific Notes
- **macOS arm64**: NEON SIMD intrinsics require explicit unsafe blocks
- **Windows x64**: CI uses `--locked` flag, Cargo.lock must match Cargo.toml
- **WASM**: Size gate checks library (~870KB), not CLI binary (~5KB)

## Common Deployment Failures & Recovery
1. **crates.io name conflicts**: Workspace crate names may be taken. Rename before first publish (e.g., `csm-core` -> `csm-core-lib`).
2. **Environment wait timer**: Default 15 min wait timer on crates.io environment. Remove timer in GitHub repo settings if unneeded.
3. **Recovery dispatch**: `gh workflow run release.yml -f recover=true` retries failed releases for existing tags idempotently.
