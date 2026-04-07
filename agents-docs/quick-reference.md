# Quick Reference Commands

## Build Performance
```bash
# sccache can be enabled for local builds (not in CI):
# Add to .cargo/config.toml:
# [build]
# rustc-wrapper = "sccache"
# Start server: sccache --start-server
# Check stats: sccache --stats

# Free disk space (removes ~35GB from target/)
cargo clean

# Rebuild faster with sccache
```

## Version Sync (Before Release)
```bash
# Check version synchronization (runs in CI)
./scripts/verify-version-sync.sh

# Sync version across all files (prevents stale docs)
./scripts/sync-version.sh 0.2.5
```

**Version must match in:**
- `Cargo.toml` - `version = "X.Y.Z"`
- `wasm/package.json` - `"version": "X.Y.Z"`
- Test fixtures (grep `"version":` in tests/ and examples/)
- npm registry (after publishing WASM package)

## Release Checklist
1. Update version in `Cargo.toml` and `wasm/package.json`
2. Run `./scripts/verify-version-sync.sh`
3. Build WASM: `cargo build --target wasm32-unknown-unknown --release --features wasm`
4. Publish crates.io: `cargo publish`
5. Publish npm: `cd wasm && npm publish`
6. Create GitHub release: `gh release create vX.Y.Z`

## Validation Gates
Run before commit (see `git-workflow` skill for details):
```bash
scripts/validate.sh
```

## Documentation Link Check
Validate links, commands, and version references in docs:
```bash
scripts/check-docs-links.sh # Quick check (links + versions)
scripts/check-docs-links.sh --fix # Auto-fix version mismatches
scripts/check-docs-links.sh --check-urls # Full URL validation
```

## Pre-Release Validation
Run before every git tag / release:
```bash
./scripts/pre-release-validate.sh # Full validation
./scripts/pre-release-validate.sh --skip-bench # Skip benchmarks (faster)
```

## Auto-generate AI docs
```bash
scripts/gen-llms-txt.sh # generates llms.txt and llms-full.txt
```
This runs automatically on post-commit when source files change.

## Performance Gate
```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```
Target: `reservoir_step_50k < 100μs`

## Commit Format
Use Conventional Commits (see `git-workflow` skill):
```
<type>(<scope>): <description>

<body>
```

## CLI Commands
```bash
# Ingest content into memory
csm index-jsonl <file.jsonl>     # Index JSONL file
csm index-dir <directory>        # Index directory of files

# Query memory
csm query "search terms"         # Text-based similarity search

# Concept operations
csm inject <id> --from-text "content"
csm probe <id> --top-k 10
csm associate <from> <to> --strength 0.8

# Export/Import
csm export > backup.json
csm import backup.json
```

## Memory Storage Paths
- **git-local mode** (default): `.csm/memory.db` in repo root
- **Custom path**: Set `CSM_DB_PATH` environment variable
