# ADR-0052: Pre-Release Validation Automation

## Status

Accepted

## Context

The README.md documents CLI commands and development gates that should work before every release. However, there was no automated way to verify all these commands actually work as documented. Issues discovered:

1. CLI commands in README were not systematically tested
2. Development gates (check, test, fmt, clippy) were run separately
3. No unified script to run all validation before a release tag
4. GOAP_STATE.md tracked `version_sync_script_created: false`

## Decision

Create a unified pre-release validation script (`scripts/pre-release-validate.sh`) that:

1. Verifies all README CLI commands work as documented:
   - `csm inject`
   - `csm probe`
   - `csm associate`
   - `csm export`
   - `csm import`
   - `csm completions`
   - `csm version`

2. Runs all development gates from README:
   - `cargo check --quiet`
   - `cargo test --all-features --quiet`
   - `cargo fmt --check --quiet`
   - `cargo clippy --quiet -- -D warnings`

3. Validates LOC policy (<= 500 lines per file)

4. Checks WASM build and size gate

5. Optionally runs benchmarks

6. Provides version sync script integration

## Usage

```bash
# Run all validation
./scripts/pre-release-validate.sh

# Skip benchmarks (faster)
./scripts/pre-release-validate.sh --skip-bench

# With version sync (for release)
./scripts/pre-release-validate.sh --version 0.2.0
```

## Integration Points

### AGENTS.md
- Added reference to pre-release-validate.sh in Quick Reference
- Run before every release tag

### release-management SKILL.md
- Pre-release validation step references this script

### GOAP_STATE.md
- Updated `version_sync_script_created: true`

## Consequences

### Positive
- Automated verification of all README commands
- Single command to validate before release
- Catches issues before they reach production
- Clear pass/fail output with summary

### Negative
- Takes ~2-3 minutes to run full validation
- Requires release binary build

## Implementation

- Created `scripts/pre-release-validate.sh`
- Updated `plans/GOAP_STATE.md`
- Updated `AGENTS.md` Quick Reference section
- Updated `.agents/skills/release-management/SKILL.md`

## References

- README.md CLI Usage section
- README.md Development Gates section
- scripts/sync-version.sh
- scripts/validate.sh