# Quick Reference — Unique Commands

Commands NOT covered by skill files or `./scripts/validate.sh`.

## Build Performance
```bash
# sccache (optional local; not in CI)
# Add to .cargo/config.toml: [build] rustc-wrapper = "sccache"
cargo clean  # frees ~35GB from target/
```

## Version Sync
```bash
./scripts/verify-version-sync.sh          # runs in CI
./scripts/sync-version.sh 0.2.5           # sync all files
./scripts/check-docs-links.sh             # link + version check
./scripts/check-docs-links.sh --fix       # auto-fix version mismatches
./scripts/check-docs-links.sh --check-urls  # full URL validation
```

## Pre-Release
```bash
./scripts/pre-release-validate.sh              # full validation
./scripts/pre-release-validate.sh --skip-bench  # faster
```

## AI Docs Generation
```bash
./scripts/gen-llms-txt.sh  # generates llms.txt + llms-full.txt
```

## Memory Storage
- **git-local mode** (default): `.csm/memory.db` in repo root
- **Custom path**: `CSM_DB_PATH` env var

## Skill Management
```bash
./scripts/setup-skills.sh            # symlinks to ~/.claude/skills/
./scripts/validate-skills.sh         # check required files
./scripts/validate-skill-format.sh   # frontmatter + section check
./scripts/validate-links.sh          # validate links in skills
```

## CI Scripts
```bash
./scripts/validate-github-actions-shas.sh  # SHA pinning check
./scripts/validate-git-hooks.sh            # hook installation check
./scripts/validate-workflows.sh            # workflow validation
```

## Performance Gate
```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
# Target: reservoir_step_50k < 100μs
```
