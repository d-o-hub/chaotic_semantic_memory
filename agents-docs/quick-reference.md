# Quick Reference Commands

## Version Sync (Before Release)
```bash
# Sync version across all files (prevents stale docs)
./scripts/sync-version.sh 0.2.0
```

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
